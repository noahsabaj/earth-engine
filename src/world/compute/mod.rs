//! GPU compute kernels and optimization structures
//!
//! This module contains all GPU-accelerated world processing systems,
//! including unified kernels, optimization structures, and effects.

mod kernels;
mod chunk_modifier;
mod optimization;
mod effects;
mod shaders;
mod unified_memory;
mod skylight;
mod gpu_lighting;
mod gpu_light_propagator;
mod weather;
mod gpu_block_query;
pub mod hierarchical_physics;
pub mod bvh;
pub mod sparse_octree;

// GPU kernels and unified systems
pub use kernels::{
    UnifiedWorldKernel, UnifiedKernelConfig, SystemFlags
};
pub use chunk_modifier::{ChunkModifier, ModificationCommand};

// GPU optimization structures
pub use hierarchical_physics::{HierarchicalPhysics, PhysicsQuery, QueryType, QueryResult};
pub use bvh::{VoxelBvh, BvhNode, BvhStats};
pub use sparse_octree::{SparseVoxelOctree, OctreeNode, OctreeStats, OctreeUpdater};

// Memory management
pub use unified_memory::{UnifiedMemoryManager, UnifiedMemoryLayout, MemoryStats};

// GPU effects and lighting
pub use effects::{
    GpuLighting, GpuLightPropagator,
    WeatherGpu, WeatherData, WeatherTransition, WeatherConfig
};

// Skylight calculation
pub use skylight::{SkylightCalculator, MAX_SKY_LIGHT};

// GPU block queries
pub use gpu_block_query::{
    GpuBlockQuery, BlockQueryRequest, BlockQueryResult, BlockQueryHandle
};

// Shader management
pub use shaders::{ShaderManager, ComputeShaderConfig, ShaderError};

/// Unified compute backend for GPU world processing
pub struct UnifiedCompute {
    device: std::sync::Arc<wgpu::Device>,
    queue: std::sync::Arc<wgpu::Queue>,
    kernel: UnifiedWorldKernel,
    memory_manager: UnifiedMemoryManager,
    shader_manager: ShaderManager,
}

impl UnifiedCompute {
    /// Create a new unified compute backend
    pub async fn new(
        device: std::sync::Arc<wgpu::Device>,
        queue: std::sync::Arc<wgpu::Queue>,
        config: UnifiedComputeConfig,
    ) -> Result<Self, ComputeError> {
        let kernel = UnifiedWorldKernel::new(device.clone(), config.kernel_config)?;
        // Use default world size for now - should come from config
        let memory_manager = unified_memory::UnifiedMemoryManager::new(device.clone(), 256, 256);
        let shader_manager = ShaderManager::new(device.clone())?;
        
        Ok(Self {
            device,
            queue,
            kernel,
            memory_manager,
            shader_manager,
        })
    }
    
    /// Execute a compute pass with the unified kernel
    pub fn execute_unified_pass(
        &mut self,
        commands: &[ComputeCommand],
    ) -> Result<(), ComputeError> {
        self.kernel.execute_pass(&self.device, &self.queue, commands.to_vec())
    }
    
    /// Get memory statistics
    pub fn memory_stats(&self) -> MemoryStats {
        // TODO: Implement proper memory stats
        MemoryStats {
            total_allocated: 0,
            voxel_data: 0,
            chunk_metadata: 0,
            lighting_data: 0,
            entity_data: 0,
            particle_data: 0,
        }
    }
    
    /// Update optimization structures
    pub fn update_optimizations(&mut self) -> Result<(), ComputeError> {
        // Update octree, BVH, and other structures
        Ok(())
    }
}

/// Configuration for unified compute backend
#[derive(Debug, Clone)]
pub struct UnifiedComputeConfig {
    pub kernel_config: UnifiedKernelConfig,
    pub memory_config: MemoryConfig,
    pub enable_optimizations: bool,
    pub enable_effects: bool,
}

impl Default for UnifiedComputeConfig {
    fn default() -> Self {
        Self {
            kernel_config: UnifiedKernelConfig::default(),
            memory_config: MemoryConfig::default(),
            enable_optimizations: true,
            enable_effects: true,
        }
    }
}

/// Memory configuration for compute backend
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub max_memory_mb: u64,
    pub chunk_cache_size: usize,
    pub optimization_memory_mb: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 1024, // 1GB default
            chunk_cache_size: 256,
            optimization_memory_mb: 256, // 256MB for optimizations
        }
    }
}

/// Unified compute commands
#[derive(Debug, Clone)]
pub enum ComputeCommand {
    GenerateTerrain {
        chunk_pos: crate::world::core::ChunkPos,
        params: crate::world::generation::TerrainParams,
    },
    ModifyVoxels {
        commands: Vec<ModificationCommand>,
    },
    UpdateLighting {
        affected_chunks: Vec<crate::world::core::ChunkPos>,
    },
    UpdatePhysics {
        simulation_time: f32,
    },
    UpdateWeather {
        weather_data: WeatherData,
    },
}

/// Compute system errors
#[derive(Debug, thiserror::Error)]
pub enum ComputeError {
    #[error("GPU compute initialization failed: {message}")]
    InitFailed { message: String },
    
    #[error("Shader compilation failed: {shader}: {error}")]
    ShaderCompilationFailed { shader: String, error: String },
    
    #[error("Memory allocation failed: {size} bytes")]
    MemoryAllocationFailed { size: u64 },
    
    #[error("Compute pass execution failed: {message}")]
    ExecutionFailed { message: String },
    
    #[error("Invalid command: {command}")]
    InvalidCommand { command: String },
    
    #[error("Shader error: {0}")]
    ShaderError(#[from] shaders::ShaderError),
}