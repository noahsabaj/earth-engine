/// GPU-powered DefaultWorldGenerator replacement
/// 
/// This replaces the CPU DefaultWorldGenerator with GPU compute shader generation
/// while maintaining the exact same WorldGenerator interface for compatibility.
/// This is the key to achieving 95% GPU / 5% CPU performance split.

use std::sync::{Arc, Mutex};
use crate::{BlockId, Chunk, ChunkPos};
use crate::world_gpu::{WorldBuffer, WorldBufferDescriptor, TerrainGenerator, TerrainParams, VoxelData};
use super::{WorldGenerator, terrain::TerrainGenerator as CpuTerrainGenerator};
use wgpu::util::DeviceExt;

/// GPU-powered replacement for DefaultWorldGenerator
/// 
/// Uses GPU compute shaders for terrain generation but maintains CPU WorldGenerator interface
/// This allows seamless replacement in existing engine code without breaking changes
pub struct GpuDefaultWorldGenerator {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    
    /// GPU infrastructure
    world_buffer: Arc<Mutex<WorldBuffer>>,
    terrain_generator: TerrainGenerator,
    
    /// CPU fallback for surface height queries (until we add GPU readback)
    cpu_terrain_gen: CpuTerrainGenerator,
    
    /// Generation parameters
    chunk_size: u32,
    seed: u32,
    
    // Block ID mappings
    grass_id: BlockId,
    dirt_id: BlockId,
    stone_id: BlockId,
    water_id: BlockId,
    sand_id: BlockId,
}

impl GpuDefaultWorldGenerator {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        seed: u32,
        grass_id: BlockId,
        dirt_id: BlockId,
        stone_id: BlockId,
        water_id: BlockId,
        sand_id: BlockId,
    ) -> Self {
        // Create WorldBuffer with safe size that fits GPU memory limits (128MB max)
        let world_buffer_desc = WorldBufferDescriptor {
            view_distance: 3, // Conservative: 7³=343 chunks, ~45MB (safe for 128MB GPU limit)
            enable_atomics: true,
            enable_readback: true,
        };
        let world_buffer = WorldBuffer::new(device.clone(), &world_buffer_desc);
        
        // Create GPU terrain generator
        let terrain_generator = TerrainGenerator::new(device.clone());
        
        // Set up terrain parameters to match DefaultWorldGenerator behavior
        let terrain_params = TerrainParams {
            seed,
            sea_level: 64.0,
            terrain_scale: 0.01,
            mountain_threshold: 0.6,
            cave_threshold: 0.3,
            ore_chances: [0.1, 0.05, 0.02, 0.01], // Coal, Iron, Gold, Diamond
        };
        terrain_generator.update_params(&queue, &terrain_params);
        
        // Keep CPU terrain generator for surface height queries
        let cpu_terrain_gen = CpuTerrainGenerator::new(seed);
        
        Self {
            device,
            queue,
            world_buffer: Arc::new(Mutex::new(world_buffer)),
            terrain_generator,
            cpu_terrain_gen,
            chunk_size: 32, // Standard chunk size
            seed,
            grass_id,
            dirt_id,
            stone_id,
            water_id,
            sand_id,
        }
    }
    
    /// Get access to the GPU WorldBuffer for lighting and other systems
    pub fn get_world_buffer(&self) -> Arc<Mutex<WorldBuffer>> {
        Arc::clone(&self.world_buffer)
    }
    
    /// Extract generated chunk data from GPU WorldBuffer
    /// This bridges GPU generation -> CPU Chunk format for compatibility
    fn extract_chunk_from_gpu(&self, chunk_pos: ChunkPos) -> Chunk {
        log::info!("[GpuDefaultWorldGenerator] Extracting chunk {:?} from GPU using REAL GPU readback", chunk_pos);
        
        // CRITICAL: Add proper GPU synchronization before attempting readback
        // This ensures all GPU compute commands have completed before reading data
        log::debug!("[GPU_SYNC] Waiting for GPU compute operations to complete before readback...");
        self.device.poll(wgpu::Maintain::Wait);
        log::debug!("[GPU_SYNC] GPU synchronization complete, proceeding with readback");
        
        // Read actual GPU-generated voxel data from WorldBuffer
        let gpu_voxels = {
            let mut world_buffer = self.world_buffer.lock().unwrap();
            match world_buffer.read_chunk_blocking(&self.device, &self.queue, chunk_pos) {
                Ok(voxels) => {
                    log::info!("[GpuDefaultWorldGenerator] Successfully read {} voxels from GPU for chunk {:?}", 
                              voxels.len(), chunk_pos);
                    voxels
                }
                Err(e) => {
                    log::error!("[GpuDefaultWorldGenerator] GPU readback failed for chunk {:?}: {}. Using CPU fallback.", 
                               chunk_pos, e);
                    // Fall back to CPU generation if GPU readback fails
                    return self.generate_cpu_fallback_chunk(chunk_pos);
                }
            }
        };
        
        // Convert GPU VoxelData format to CPU Chunk format
        let mut chunk = Chunk::new(chunk_pos, self.chunk_size);
        
        // COMPREHENSIVE DATA VERIFICATION: Check if GPU generation actually produced terrain
        let mut total_non_air_count = 0;
        let mut total_solid_blocks = 0;
        let mut block_type_counts = std::collections::HashMap::new();
        
        // Count all non-air blocks in the entire chunk
        for gpu_voxel in gpu_voxels.iter() {
            let block_id = gpu_voxel.block_id();
            if block_id != 0 { // 0 = AIR block
                total_non_air_count += 1;
                total_solid_blocks += 1;
                *block_type_counts.entry(block_id).or_insert(0) += 1;
            }
        }
        
        // Log comprehensive statistics
        log::info!("[DATA_VERIFICATION] Chunk {:?}: {}/{} voxels are non-air ({:.1}%)", 
                  chunk_pos, total_non_air_count, gpu_voxels.len(), 
                  (total_non_air_count as f32 / gpu_voxels.len() as f32) * 100.0);
        
        if !block_type_counts.is_empty() {
            log::info!("[DATA_VERIFICATION] Block types found: {:?}", block_type_counts);
        }
        
        // Debug: Check first few raw voxel values and block IDs
        let mut non_zero_count = 0;
        let mut non_air_count = 0;
        for (i, gpu_voxel) in gpu_voxels.iter().take(10).enumerate() {
            if gpu_voxel.0 != 0 {
                non_zero_count += 1;
            }
            if gpu_voxel.block_id() != 0 {
                non_air_count += 1;
            }
            if i < 3 || gpu_voxel.0 != 0 || gpu_voxel.block_id() != 0 {
                log::debug!("[GPU→CPU] Voxel {}: raw=0x{:08x}, block_id={}", i, gpu_voxel.0, gpu_voxel.block_id());
            }
        }
        log::debug!("[GPU→CPU] Chunk {:?}: {} non-zero raw, {} non-air blocks (of first 10 voxels)", 
                   chunk_pos, non_zero_count, non_air_count);
        
        // CRITICAL: Detect if GPU generation failed for chunks that should contain terrain
        // Chunks at Y=0,1,2 (ground level) should contain significant terrain
        let expected_terrain = chunk_pos.y >= 0 && chunk_pos.y <= 2;
        if expected_terrain && total_non_air_count == 0 {
            log::error!("[GPU_GENERATION_FAILURE] Chunk {:?} at ground level contains NO terrain blocks! GPU generation likely failed.", chunk_pos);
            log::error!("[GPU_GENERATION_FAILURE] Expected terrain at ground level (Y={}) but got {} solid blocks. Falling back to CPU.", 
                       chunk_pos.y, total_non_air_count);
            return self.generate_cpu_fallback_chunk(chunk_pos);
        }
        
        // Warn if terrain density seems suspiciously low for ground-level chunks
        if expected_terrain && total_non_air_count < 1000 { // Less than ~3% filled
            log::warn!("[GPU_DATA_QUALITY] Chunk {:?} has suspiciously low terrain density: {} solid blocks ({:.1}%). Possible GPU generation issue.", 
                      chunk_pos, total_non_air_count, (total_non_air_count as f32 / gpu_voxels.len() as f32) * 100.0);
        }
        
        for (voxel_index, gpu_voxel) in gpu_voxels.iter().enumerate() {
            // Convert linear index to 3D coordinates
            let x = (voxel_index as u32) % self.chunk_size;
            let y = ((voxel_index as u32) / self.chunk_size) % self.chunk_size;
            let z = (voxel_index as u32) / (self.chunk_size * self.chunk_size);
            
            // Extract block data from GPU voxel format
            let block_id = BlockId(gpu_voxel.block_id());
            let light_level = gpu_voxel.light_level();
            let sky_light = gpu_voxel.sky_light_level();
            
            // Set block data in CPU chunk
            if block_id != BlockId::AIR {
                chunk.set_block(x, y, z, block_id);
            }
            chunk.set_block_light(x, y, z, light_level);
            chunk.set_sky_light(x, y, z, sky_light);
        }
        
        log::debug!("[GpuDefaultWorldGenerator] Converted {} GPU voxels to CPU chunk format for {:?}", 
                   gpu_voxels.len(), chunk_pos);
        
        chunk
    }
    
    /// CPU fallback for when GPU readback fails
    /// This generates simple terrain using the CPU terrain generator
    fn generate_cpu_fallback_chunk(&self, chunk_pos: ChunkPos) -> Chunk {
        log::error!("[CPU_FALLBACK] Using CPU fallback for chunk {:?} - GPU generation failed", chunk_pos);
        
        let mut chunk = Chunk::new(chunk_pos, self.chunk_size);
        
        let world_x_start = chunk_pos.x * self.chunk_size as i32;
        let world_y_start = chunk_pos.y * self.chunk_size as i32;
        let world_z_start = chunk_pos.z * self.chunk_size as i32;
        
        // Generate basic terrain using CPU terrain generator
        for x in 0..self.chunk_size {
            for z in 0..self.chunk_size {
                let world_x = world_x_start + x as i32;
                let world_z = world_z_start + z as i32;
                
                let surface_height = self.cpu_terrain_gen.get_height(world_x as f64, world_z as f64);
                
                for y in 0..self.chunk_size {
                    let world_y = world_y_start + y as i32;
                    
                    if world_y <= surface_height {
                        let block_id = if world_y == surface_height {
                            if surface_height < 64 { self.sand_id } else { self.grass_id }
                        } else if world_y > surface_height - 4 {
                            self.dirt_id
                        } else {
                            self.stone_id
                        };
                        chunk.set_block(x, y, z, block_id);
                    }
                }
                
                // Add water at sea level
                if surface_height < 64 {
                    for y in 0..self.chunk_size {
                        let world_y = world_y_start + y as i32;
                        if world_y > surface_height && world_y <= 64 {
                            if chunk.get_block(x, y, z) == BlockId::AIR {
                                chunk.set_block(x, y, z, self.water_id);
                            }
                        }
                    }
                }
            }
        }
        
        // Set basic skylight
        for x in 0..self.chunk_size {
            for z in 0..self.chunk_size {
                for y in (0..self.chunk_size).rev() {
                    if chunk.get_block(x, y, z) == BlockId::AIR {
                        chunk.set_sky_light(x, y, z, 15);
                    } else {
                        chunk.set_sky_light(x, y, z, 0);
                        break;
                    }
                }
            }
        }
        
        chunk
    }
    
    /// Generate chunk on GPU and extract to CPU format
    fn generate_chunk_gpu(&self, chunk_pos: ChunkPos) -> Chunk {
        log::info!("[GpuDefaultWorldGenerator] GPU-generating chunk {:?} (TESTING SELF-CONTAINED SHADER)", chunk_pos);
        
        // Log chunk context for debugging
        let world_x = chunk_pos.x * self.chunk_size as i32;
        let world_y = chunk_pos.y * self.chunk_size as i32;
        let world_z = chunk_pos.z * self.chunk_size as i32;
        log::debug!("[GPU_GENERATION] Chunk {:?} world coordinates: ({}, {}, {}) to ({}, {}, {})", 
                   chunk_pos, world_x, world_y, world_z, 
                   world_x + self.chunk_size as i32 - 1, 
                   world_y + self.chunk_size as i32 - 1, 
                   world_z + self.chunk_size as i32 - 1);
        
        // GPU GENERATION RE-ENABLED - NVIDIA GPU now properly detected and selected
        log::info!("[GpuDefaultWorldGenerator] GPU generation ENABLED - using NVIDIA RTX 4060 Ti hardware acceleration");
        log::debug!("[GPU_COMMANDS] Starting GPU command recording for chunk {:?}", chunk_pos);
        
        {
            // Create command encoder for GPU operations
            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GPU Terrain Generation"),
            });
            
            log::debug!("[GPU_COMMANDS] Created command encoder, acquiring WorldBuffer lock...");
            
            // Generate chunk directly in WorldBuffer using compute shader on NVIDIA GPU
            {
                let mut world_buffer = self.world_buffer.lock().unwrap();
                log::debug!("[GPU_COMMANDS] WorldBuffer locked, dispatching compute shader...");
                
                self.terrain_generator.generate_chunk(
                    &mut encoder,
                    &mut world_buffer,
                    chunk_pos,
                );
                
                log::debug!("[GPU_COMMANDS] Compute shader dispatch complete for chunk {:?}", chunk_pos);
            }
            
            // Submit GPU commands to NVIDIA hardware
            log::debug!("[GPU_COMMANDS] Submitting command buffer to GPU queue...");
            self.queue.submit(std::iter::once(encoder.finish()));
            log::debug!("[GPU_COMMANDS] GPU commands submitted successfully for chunk {:?}", chunk_pos);
        }
        
        // CRITICAL: Ensure GPU generation is 100% complete before readback
        log::debug!("[GPU_SYNC] Ensuring GPU terrain generation completes before readback...");
        self.device.poll(wgpu::Maintain::Wait);
        log::debug!("[GPU_SYNC] GPU generation guaranteed complete for chunk {:?}", chunk_pos);
        
        log::debug!("[GPU_GENERATION] GPU command submission complete, proceeding to readback...");
        
        // Extract the generated chunk data back to CPU format
        // This maintains compatibility with existing engine systems
        self.extract_chunk_from_gpu(chunk_pos)
    }
}

impl WorldGenerator for GpuDefaultWorldGenerator {
    fn generate_chunk(&self, chunk_pos: ChunkPos, chunk_size: u32) -> Chunk {
        assert_eq!(chunk_size, self.chunk_size, "Chunk size mismatch in GPU generator");
        
        log::debug!("[GpuDefaultWorldGenerator] Generating chunk {:?} using GPU compute shader", chunk_pos);
        
        // GPU generation with CPU extraction for compatibility
        self.generate_chunk_gpu(chunk_pos)
    }
    
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        // Delegate to CPU terrain generator for now
        // TODO: Add GPU compute shader for surface height queries
        self.cpu_terrain_gen.get_height(world_x, world_z)
    }
    
    fn get_world_buffer(&self) -> Option<std::sync::Arc<std::sync::Mutex<crate::world_gpu::WorldBuffer>>> {
        Some(Arc::clone(&self.world_buffer))
    }
}

/// Create GPU-powered DefaultWorldGenerator with default block mappings
pub fn create_gpu_default_world_generator(
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    seed: u32,
) -> GpuDefaultWorldGenerator {
    GpuDefaultWorldGenerator::new(
        device,
        queue,
        seed,
        BlockId::GRASS,
        BlockId::DIRT,
        BlockId::STONE,
        BlockId::WATER,
        BlockId::SAND,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gpu_default_world_generator_creation() {
        // This test requires GPU context, so we'll skip in normal test runs
        // TODO: Add proper GPU testing infrastructure
    }
}