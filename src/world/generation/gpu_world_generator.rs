//! GPU world generator wrapper that implements the WorldGenerator trait

use crate::gpu::{GpuError, GpuErrorRecovery, GpuRecoveryError};
use crate::world::{
    core::{BlockId, ChunkPos},
    generation::{TerrainGeneratorSOA, WorldGenerator},
    storage::{ChunkSoA, WorldBuffer},
};
use std::sync::{Arc, Mutex};

/// GPU world generator that wraps TerrainGeneratorSOA to implement WorldGenerator trait
///
/// This is a wrapper that defers actual GPU generation until a proper command encoder
/// is available. The WorldGenerator trait doesn't provide access to command encoders,
/// so this wrapper stores the generation parameters and performs the actual generation
/// when the renderer provides an encoder.
pub struct GpuWorldGenerator {
    terrain_generator: Arc<TerrainGeneratorSOA>,
    device: Arc<wgpu::Device>,
    world_buffer: Arc<Mutex<WorldBuffer>>,
    error_recovery: Arc<GpuErrorRecovery>,
}

impl GpuWorldGenerator {
    /// Create a new GPU world generator
    pub fn new(
        terrain_generator: Arc<TerrainGeneratorSOA>,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        world_buffer: Arc<Mutex<WorldBuffer>>,
    ) -> Self {
        let error_recovery = Arc::new(GpuErrorRecovery::new(device.clone(), queue));

        Self {
            terrain_generator,
            device,
            world_buffer,
            error_recovery,
        }
    }

    /// Generate chunks on GPU when a command encoder is available
    /// This is the proper way to use GPU generation
    pub fn generate_chunks_with_encoder(
        &self,
        chunk_positions: &[ChunkPos],
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), GpuError> {
        // Check if device is lost before proceeding
        if self.error_recovery.is_device_lost() {
            return Err(GpuError::DeviceLost);
        }

        // Validate encoder before use
        if let Err(e) = self.error_recovery.validate_encoder(encoder) {
            log::error!("Command encoder is invalid: {:?}", e);
            return Err(GpuError::InvalidEncoder);
        }

        // Execute terrain generation with error recovery
        let result = self.error_recovery.execute_with_recovery(|| {
            let mut world_buffer = self.world_buffer.lock().unwrap();
            self.terrain_generator
                .generate_chunks(&mut world_buffer, chunk_positions, encoder)
                .map_err(|gpu_err| GpuRecoveryError::OperationFailed {
                    message: format!("Terrain generation failed: {:?}", gpu_err),
                })
        });

        match result {
            Ok(_metadata_buffer) => Ok(()),
            Err(GpuRecoveryError::DeviceLost) => Err(GpuError::DeviceLost),
            Err(GpuRecoveryError::TooManyErrors { count }) => {
                log::error!("Too many GPU errors during terrain generation: {}", count);
                Err(GpuError::TooManyErrors)
            }
            Err(GpuRecoveryError::Panic { message }) => {
                log::error!("GPU operation panicked: {}", message);
                Err(GpuError::GpuPanic)
            }
            Err(e) => {
                log::error!("GPU error during terrain generation: {:?}", e);
                Err(GpuError::Other(format!("{:?}", e)))
            }
        }
    }
}

impl WorldGenerator for GpuWorldGenerator {
    fn generate_chunk(&self, chunk_pos: ChunkPos, chunk_size: u32) -> ChunkSoA {
        // The WorldGenerator trait doesn't provide access to command encoders,
        // so we can't perform GPU generation through this synchronous interface.
        // This is a fundamental limitation of the current architecture.
        //
        // For now, we return an empty chunk and log a warning.
        // The proper way to use GPU generation is through generate_chunks_with_encoder
        // when a command encoder is available.
        log::warn!(
            "GPU generation requested through synchronous interface for chunk {:?}. \
             Returning empty chunk. Use generate_chunks_with_encoder for proper GPU generation.",
            chunk_pos
        );
        ChunkSoA::new(chunk_pos, chunk_size)
    }

    fn get_surface_height(&self, world_x: f64, world_z: f64) -> i32 {
        // Use the constant from the root constants.rs file
        use crate::terrain::SEA_LEVEL;
        SEA_LEVEL as i32
    }

    fn is_gpu(&self) -> bool {
        true
    }

    fn get_world_buffer(&self) -> Option<Arc<Mutex<WorldBuffer>>> {
        Some(self.world_buffer.clone())
    }
}
