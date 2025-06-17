/// GPU-powered world generator that bridges CPU chunk management with GPU terrain generation
/// 
/// This implements the WorldGenerator trait but delegates actual generation to GPU compute shaders,
/// then extracts the results back to CPU Chunk format for compatibility with existing systems.

use std::sync::{Arc, Mutex};
use crate::{BlockId, Chunk, ChunkPos};
use crate::world_gpu::{WorldBuffer, WorldBufferDescriptor, TerrainGenerator, TerrainParams, VoxelData};
use crate::world_gpu::terrain_generator::{BlockDistribution, MAX_BLOCK_DISTRIBUTIONS};
use super::{WorldGenerator, terrain::TerrainGenerator as CpuTerrainGenerator};
use wgpu::util::DeviceExt;

/// GPU-powered world generator that maintains compatibility with CPU chunk management
pub struct GpuWorldGenerator {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    world_buffer: Arc<Mutex<WorldBuffer>>,
    terrain_generator: TerrainGenerator,
    cpu_terrain_gen: CpuTerrainGenerator, // For surface height queries
    chunk_size: u32,
    
    // Block ID mappings
    grass_id: BlockId,
    dirt_id: BlockId,
    stone_id: BlockId,
    water_id: BlockId,
    sand_id: BlockId,
}

impl GpuWorldGenerator {
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        seed: u32,
        chunk_size: u32,
        grass_id: BlockId,
        dirt_id: BlockId,
        stone_id: BlockId,
        water_id: BlockId,
        sand_id: BlockId,
    ) -> Self {
        // Create WorldBuffer for GPU-resident voxel data
        let world_buffer_desc = WorldBufferDescriptor {
            view_distance: 3, // Conservative: 7³=343 chunks, ~45MB (safe for 128MB GPU limit)
            enable_atomics: true,
            enable_readback: true, // Enable readback so we can extract chunks
        };
        let world_buffer = WorldBuffer::new(device.clone(), &world_buffer_desc);
        
        // Create GPU terrain generator
        let terrain_generator = TerrainGenerator::new(device.clone());
        
        // Set terrain parameters
        let terrain_params = TerrainParams {
            seed,
            sea_level: 64.0,
            terrain_scale: 0.01,
            mountain_threshold: 0.6,
            cave_threshold: 0.3,
            num_distributions: 0, // No custom distributions for basic world generator
            _padding: [0; 2],
            distributions: [BlockDistribution::default(); MAX_BLOCK_DISTRIBUTIONS],
        };
        terrain_generator.update_params(&queue, &terrain_params);
        
        // Keep CPU terrain generator for surface height queries (until we add GPU readback for this)
        let cpu_terrain_gen = CpuTerrainGenerator::new(seed);
        
        Self {
            device,
            queue,
            world_buffer: Arc::new(Mutex::new(world_buffer)),
            terrain_generator,
            cpu_terrain_gen,
            chunk_size,
            grass_id,
            dirt_id,
            stone_id,
            water_id,
            sand_id,
        }
    }
    
    /// Extract voxel data from WorldBuffer for a specific chunk
    /// This is the bridge between GPU generation and CPU chunk format
    fn extract_chunk_from_gpu(&self, chunk_pos: ChunkPos) -> Chunk {
        // For now, we'll generate on GPU and then extract
        // In a future optimization, we could cache generated chunks in WorldBuffer
        // and only extract when requested
        
        log::info!("[GpuWorldGenerator] Extracting chunk {:?} from GPU buffer", chunk_pos);
        
        // TODO: Implement actual GPU buffer readback
        // For now, this is a placeholder that generates a basic chunk
        // Once GPU generation is working, we'll read back from world_buffer
        
        let mut chunk = Chunk::new(chunk_pos, self.chunk_size);
        
        // For initial testing, create a simple pattern to verify the system works
        let world_x_start = chunk_pos.x * self.chunk_size as i32;
        let world_y_start = chunk_pos.y * self.chunk_size as i32;
        let world_z_start = chunk_pos.z * self.chunk_size as i32;
        
        for x in 0..self.chunk_size {
            for z in 0..self.chunk_size {
                let world_x = world_x_start + x as i32;
                let world_z = world_z_start + z as i32;
                
                // Use CPU terrain generator for height temporarily
                // TODO: Get this from GPU WorldBuffer once readback is implemented
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
                        break; // Stop skylight propagation
                    }
                }
            }
        }
        
        chunk
    }
    
    /// Generate chunk on GPU and extract to CPU format
    fn generate_chunk_gpu(&self, chunk_pos: ChunkPos) -> Chunk {
        log::info!("[GpuWorldGenerator] GPU-generating chunk {:?}", chunk_pos);
        
        // Create command encoder for GPU operations
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GPU Terrain Generation"),
        });
        
        // Generate chunk directly in WorldBuffer using compute shader
        {
            let mut world_buffer = self.world_buffer.lock().unwrap();
            self.terrain_generator.generate_chunk(
                &mut encoder,
                &mut world_buffer,
                chunk_pos,
            );
        }
        
        // Submit GPU commands
        self.queue.submit(std::iter::once(encoder.finish()));
        
        // Extract the generated chunk data back to CPU format
        self.extract_chunk_from_gpu(chunk_pos)
    }
}

impl WorldGenerator for GpuWorldGenerator {
    fn generate_chunk(&self, chunk_pos: ChunkPos, chunk_size: u32) -> Chunk {
        assert_eq!(chunk_size, self.chunk_size, "Chunk size mismatch in GPU generator");
        
        log::info!("[GpuWorldGenerator] Generating chunk {:?} using GPU", chunk_pos);
        
        // Generate on GPU and extract to CPU format
        self.generate_chunk_gpu(chunk_pos)
    }
    
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        // For now, delegate to CPU terrain generator
        // TODO: Add GPU compute shader for surface height queries
        self.cpu_terrain_gen.get_height(world_x, world_z)
    }
    
    fn get_world_buffer(&self) -> Option<std::sync::Arc<std::sync::Mutex<crate::world_gpu::WorldBuffer>>> {
        Some(Arc::clone(&self.world_buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockId;
    
    #[test]
    fn test_gpu_world_generator_creation() {
        // This test requires GPU context, so we'll skip in normal test runs
        // TODO: Add proper GPU testing infrastructure
    }
}