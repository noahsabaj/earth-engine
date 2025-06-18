//! GPU compute kernels and optimization structures
//!
//! This module contains all GPU-accelerated world processing systems,
//! including unified kernels, optimization structures, and effects.

mod kernels;
mod optimization;
mod effects;
mod shaders;

// GPU kernels and unified systems
pub use kernels::{
    UnifiedWorldKernel, UnifiedKernelConfig, SystemFlags,
    ChunkModifier, ModificationCommand
};

// GPU optimization structures
pub use optimization::{
    SparseVoxelOctree, OctreeNode, OctreeStats, OctreeUpdater,
    VoxelBvh, BvhNode, BvhStats,
    HierarchicalPhysics, PhysicsQuery, QueryResult, QueryType,
    UnifiedMemoryManager, UnifiedMemoryLayout, MemoryStats
};

// GPU effects and lighting
pub use effects::{
    GpuLighting, GpuLightPropagator,
    WeatherGpu, WeatherData, WeatherTransition, WeatherConfig
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
        let memory_manager = UnifiedMemoryManager::new(device.clone(), config.memory_config)?;
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
        commands: &[UnifiedCommand],
    ) -> Result<(), ComputeError> {
        self.kernel.execute_pass(&self.device, &self.queue, commands)
    }
    
    /// Get memory statistics
    pub fn memory_stats(&self) -> MemoryStats {
        self.memory_manager.get_stats()
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
pub enum UnifiedCommand {
    GenerateTerrain {
        chunk_pos: crate::world_unified::core::ChunkPos,
        params: crate::world_unified::generation::TerrainParams,
    },
    ModifyVoxels {
        commands: Vec<ModificationCommand>,
    },
    UpdateLighting {
        affected_chunks: Vec<crate::world_unified::core::ChunkPos>,
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
}