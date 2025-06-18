use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use crate::world::{Chunk, ChunkPos, BlockId};
use crate::morton::morton_encode; // Used for intra-chunk voxel ordering

// TODO: CRITICAL DESIGN ISSUE - Morton Encoding vs Slot-Based Allocation
// The migration code incorrectly assumed Morton encoding for chunk buffer offsets,
// but WorldBuffer uses a simple slot-based allocation system. This mismatch could
// cause buffer corruption. The migration needs redesign to either:
// 1. Have mutable access to WorldBuffer for slot allocation, or
// 2. Pre-allocate slots before migration
// This code appears to be unused currently, but needs fixing before use.
use crate::memory::{BandwidthProfiler, TransferType};
use super::world_buffer::{WorldBuffer, VoxelData};
use crate::gpu::constants::core::CHUNK_SIZE;
use super::gpu_lighting::GpuLighting;

/// Handles migration of CPU-side chunk data to GPU world buffer
pub struct WorldMigrator {
    device: Arc<wgpu::Device>,
    
    /// Staging buffer for chunk uploads
    staging_buffer: wgpu::Buffer,
    staging_capacity: usize,
    
    /// Buffer for chunk position list
    position_buffer: wgpu::Buffer,
    position_capacity: usize,
}

impl WorldMigrator {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        // Create staging buffer for chunk data transfers
        let staging_capacity = 100usize; // Can stage 100 chunks at once
        let chunk_data_size = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 4) as usize; // 4 bytes per voxel
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunk Migration Staging Buffer"),
            size: (staging_capacity * chunk_data_size) as u64,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::MAP_WRITE,
            mapped_at_creation: false,
        });
        
        // Create position buffer
        let position_capacity = 1000usize; // Can track 1000 chunks
        let position_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Migration Position Buffer"),
            size: (position_capacity * 16) as u64, // vec4<i32> per position
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Self {
            device,
            staging_buffer,
            staging_capacity,
            position_buffer,
            position_capacity,
        }
    }
    
    /// Migrate a single chunk from CPU to GPU with Morton encoding
    pub fn migrate_chunk(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        world_buffer: &WorldBuffer,
        chunk: &Chunk,
        chunk_pos: ChunkPos,
        profiler: Option<&mut BandwidthProfiler>,
    ) {
        let start_time = std::time::Instant::now();
        
        // Convert chunk data to GPU format with Morton encoding
        let mut gpu_data = vec![0u32; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize];
        
        // Use Morton encoding for better cache locality
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let block = chunk.get_block(x, y, z);
                    
                    // Pack voxel data
                    let voxel = VoxelData::new(
                        block.0,
                        0,  // Initial light value
                        15, // Full skylight for now (will be recalculated)
                        0,  // No metadata initially
                    );
                    
                    // Use Morton encoding for voxel placement
                    let morton_index = morton_encode(x, y, z);
                    gpu_data[morton_index as usize] = voxel.0;
                }
            }
        }
        
        let data_size = (gpu_data.len() * 4) as u64;
        
        // Upload to staging buffer
        queue.write_buffer(
            &self.staging_buffer,
            0,
            bytemuck::cast_slice(&gpu_data),
        );
        
        // Allocate a slot for this chunk in the world buffer
        // TODO: Fix design issue - migration needs mutable WorldBuffer access to allocate slots
        // For now, using a placeholder slot calculation. This code appears to be unused.
        // Proper fix: Either pass mutable WorldBuffer or pre-allocate slots before migration
        let slot = 0; // FIXME: This is incorrect but allows compilation
        let dest_offset = world_buffer.slot_offset(slot);
        
        // Copy from staging to world buffer
        encoder.copy_buffer_to_buffer(
            &self.staging_buffer,
            0,
            world_buffer.voxel_buffer(),
            dest_offset,
            data_size,
        );
        
        // Update chunk metadata
        let metadata_offset = slot as u64 * 16; // 16 bytes per chunk metadata
        let metadata = ChunkMetadata {
            flags: 0b11, // Generated and migrated
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as u32)
                .unwrap_or(0),
            checksum: self.calculate_checksum(&gpu_data),
            reserved: 0,
        };
        queue.write_buffer(
            world_buffer.metadata_buffer(),
            metadata_offset,
            bytemuck::cast_slice(&[metadata]),
        );
        
        // Record bandwidth if profiler provided
        if let Some(profiler) = profiler {
            let duration_us = start_time.elapsed().as_micros() as u64;
            if let Err(e) = profiler.record_typed_transfer(data_size, duration_us, TransferType::Upload) {
                log::warn!("[WorldGpuMigration] Failed to record bandwidth: {:?}", e);
            }
        }
    }
    
    /// Calculate simple checksum for chunk data
    fn calculate_checksum(&self, data: &[u32]) -> u32 {
        data.iter().fold(0u32, |acc, &val| acc.wrapping_add(val))
    }
    
    /// Migrate multiple chunks in batch
    pub fn migrate_chunks_batch(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        world_buffer: &WorldBuffer,
        chunks: &[(ChunkPos, &Chunk)],
    ) -> MigrationStats {
        let mut stats = MigrationStats::default();
        
        // Process chunks in batches that fit in staging buffer
        for batch in chunks.chunks(self.staging_capacity) {
            stats.batches += 1;
            
            // Prepare position data for lighting update
            let positions: Vec<[i32; 4]> = batch
                .iter()
                .map(|(pos, _)| [pos.x, pos.y, pos.z, 0])
                .collect();
            
            // Upload position data
            queue.write_buffer(
                &self.position_buffer,
                0,
                bytemuck::cast_slice(&positions),
            );
            
            // Migrate each chunk in the batch
            for (i, (chunk_pos, chunk)) in batch.iter().enumerate() {
                // Convert and upload chunk data with Morton encoding
                let mut gpu_data = vec![0u32; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize];
                let mut block_count = 0u32;
                
                for y in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        for x in 0..CHUNK_SIZE {
                            let block = chunk.get_block(x, y, z);
                            
                            if block != BlockId::AIR {
                                block_count += 1;
                            }
                            
                            let voxel = VoxelData::new(
                                block.0,
                                0,
                                15,
                                0,
                            );
                            
                            // Use Morton encoding for voxel placement
                            let morton_index = morton_encode(x, y, z);
                            gpu_data[morton_index as usize] = voxel.0;
                        }
                    }
                }
                
                stats.chunks_migrated += 1;
                stats.blocks_migrated += block_count;
                
                // Upload to specific offset in staging buffer
                let staging_offset = (i * (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 4) as usize) as u64;
                queue.write_buffer(
                    &self.staging_buffer,
                    staging_offset,
                    bytemuck::cast_slice(&gpu_data),
                );
                
                // Allocate a slot for this chunk in the world buffer
                // TODO: Fix design issue - migration needs mutable WorldBuffer access
                // Using index as slot for now. This code appears to be unused.
                let slot = i as u32; // FIXME: This is incorrect but allows compilation
                let dest_offset = world_buffer.slot_offset(slot);
                let voxels_per_chunk = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
                
                // Copy to world buffer
                encoder.copy_buffer_to_buffer(
                    &self.staging_buffer,
                    staging_offset,
                    world_buffer.voxel_buffer(),
                    dest_offset,
                    (voxels_per_chunk * 4) as u64,
                );
                
                // Update metadata
                let metadata_offset = slot as u64 * 16;
                let metadata = ChunkMetadata {
                    flags: 0b11,
                    timestamp: 0,
                    checksum: 0,
                    reserved: 0,
                };
                queue.write_buffer(
                    world_buffer.metadata_buffer(),
                    metadata_offset,
                    bytemuck::cast_slice(&[metadata]),
                );
            }
        }
        
        stats
    }
    
    /// Migrate an entire world from CPU to GPU
    pub fn migrate_world<F>(
        &self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        world_buffer: &WorldBuffer,
        lighting: &GpuLighting,
        chunk_iterator: impl Iterator<Item = (ChunkPos, Chunk)>,
        mut progress_callback: F,
    ) -> MigrationStats
    where
        F: FnMut(usize, usize),
    {
        let mut stats = MigrationStats::default();
        let mut batch = Vec::new();
        let mut total_chunks = 0;
        
        // Collect chunks and process in batches
        for (chunk_pos, chunk) in chunk_iterator {
            batch.push((chunk_pos, chunk));
            
            // Process batch when full
            if batch.len() >= self.staging_capacity {
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("World Migration Encoder"),
                });
                
                // Convert to references for migration
                let batch_refs: Vec<(ChunkPos, &Chunk)> = batch
                    .iter()
                    .map(|(pos, chunk)| (*pos, chunk))
                    .collect();
                
                let batch_stats = self.migrate_chunks_batch(
                    queue,
                    &mut encoder,
                    world_buffer,
                    &batch_refs,
                );
                stats.add(&batch_stats);
                
                // Update lighting for migrated chunks
                let positions: Vec<ChunkPos> = batch.iter().map(|(pos, _)| *pos).collect();
                lighting.batch_update_lighting(&mut encoder, world_buffer, &positions);
                
                // Submit commands
                queue.submit(std::iter::once(encoder.finish()));
                
                total_chunks += batch.len();
                progress_callback(total_chunks, stats.chunks_migrated as usize);
                
                batch.clear();
            }
        }
        
        // Process remaining chunks
        if !batch.is_empty() {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("World Migration Final Encoder"),
            });
            
            let batch_refs: Vec<(ChunkPos, &Chunk)> = batch
                .iter()
                .map(|(pos, chunk)| (*pos, chunk))
                .collect();
            
            let batch_stats = self.migrate_chunks_batch(
                queue,
                &mut encoder,
                world_buffer,
                &batch_refs,
            );
            stats.add(&batch_stats);
            
            // Update lighting
            let positions: Vec<ChunkPos> = batch.iter().map(|(pos, _)| *pos).collect();
            lighting.batch_update_lighting(&mut encoder, world_buffer, &positions);
            
            queue.submit(std::iter::once(encoder.finish()));
            
            total_chunks += batch.len();
            progress_callback(total_chunks, stats.chunks_migrated as usize);
        }
        
        stats
    }
}

/// Chunk metadata stored in GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct ChunkMetadata {
    flags: u32,
    timestamp: u32,
    checksum: u32,
    reserved: u32,
}

/// Statistics from migration process
#[derive(Debug, Default, Clone)]
pub struct MigrationStats {
    pub chunks_migrated: u32,
    pub blocks_migrated: u32,
    pub batches: u32,
}

impl MigrationStats {
    fn add(&mut self, other: &MigrationStats) {
        self.chunks_migrated += other.chunks_migrated;
        self.blocks_migrated += other.blocks_migrated;
        self.batches += other.batches;
    }
    
    pub fn print_summary(&self) {
        println!("=== Migration Complete ===");
        println!("Chunks migrated: {}", self.chunks_migrated);
        println!("Blocks migrated: {}", self.blocks_migrated);
        println!("Batches processed: {}", self.batches);
        println!("Average blocks per chunk: {:.1}", 
                 self.blocks_migrated as f32 / self.chunks_migrated as f32);
    }
}