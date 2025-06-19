//! Adapter to make unified generators work with legacy code

use std::sync::Arc;
use crate::world_unified::generation::{UnifiedGenerator, GeneratorConfig, BlockIds, TerrainParams};

/// Adapter to make UnifiedGenerator compatible with legacy world::WorldGenerator trait
#[cfg(feature = "legacy-world-modules")]
pub struct LegacyGeneratorAdapter {
    generator: UnifiedGenerator,
}

#[cfg(feature = "legacy-world-modules")]
impl LegacyGeneratorAdapter {
    /// Create a new GPU-based legacy adapter
    pub async fn new_gpu(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        seed: u64,
        chunk_size: u32,
        grass_id: crate::BlockId,
        dirt_id: crate::BlockId,
        stone_id: crate::BlockId,
        water_id: crate::BlockId,
        sand_id: crate::BlockId,
    ) -> Self {
        let buffer_manager = Arc::new(crate::gpu::GpuBufferManager::new(device.clone(), queue));
        
        let config = GeneratorConfig {
            terrain_params: TerrainParams {
                seed: seed as u32,
                ..Default::default()
            },
            block_ids: BlockIds {
                grass: grass_id,
                dirt: dirt_id,
                stone: stone_id,
                water: water_id,
                sand: sand_id,
            },
            use_vectorization: true,
        };
        
        let generator = UnifiedGenerator::new_gpu(device, buffer_manager, config)
            .await
            .unwrap_or_else(|_| UnifiedGenerator::new_cpu(config).unwrap());
        
        Self { generator }
    }
}

#[cfg(feature = "legacy-world-modules")]
impl crate::world_unified::generation::WorldGenerator for LegacyGeneratorAdapter {
    fn generate_chunk(&self, chunk_pos: crate::ChunkPos, chunk_size: u32) -> crate::world_unified::storage::ChunkSoA {
        // Generate using unified generator directly
        self.generator.generate_chunk(chunk_pos, chunk_size)
    }
    
    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        self.generator.get_surface_height(world_x, world_z)
    }
    
    fn is_gpu(&self) -> bool {
        self.generator.is_gpu()
    }
    
    fn get_world_buffer(&self) -> Option<std::sync::Arc<std::sync::Mutex<crate::world_unified::storage::WorldBuffer>>> {
        self.generator.get_world_buffer()
    }
}

/// Create a default GPU world generator that's compatible with legacy code
#[cfg(feature = "legacy-world-modules")]
pub fn create_legacy_gpu_generator(
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    seed: u64,
    chunk_size: u32,
    grass_id: crate::BlockId,
    dirt_id: crate::BlockId, 
    stone_id: crate::BlockId,
    water_id: crate::BlockId,
    sand_id: crate::BlockId,
) -> Box<dyn crate::world_unified::generation::WorldGenerator + Send + Sync> {
    // Create the adapter synchronously using blocking
    let runtime = tokio::runtime::Handle::try_current()
        .unwrap_or_else(|_| {
            // If no runtime exists, create a minimal one
            tokio::runtime::Runtime::new().unwrap().handle().clone()
        });
    
    let adapter = runtime.block_on(LegacyGeneratorAdapter::new_gpu(
        device,
        queue,
        seed,
        chunk_size,
        grass_id,
        dirt_id,
        stone_id,
        water_id,
        sand_id,
    ));
    
    Box::new(adapter)
}