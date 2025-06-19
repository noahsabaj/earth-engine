//! Global GPU type registry initialization
//! 
//! This module initializes and manages the unified GPU type system for the entire engine

use crate::gpu::automation::{
    UnifiedGpuSystem,
    auto_wgsl::AutoWgsl,
    auto_layout::AutoLayout,
};
use crate::gpu::soa::{TerrainParamsSOA, BlockDistributionSOA};
use crate::gpu::types::world::{ChunkMetadata, VoxelData};
// TODO: Add these when they implement AutoWgsl
// use crate::gpu::buffer_layouts::{InstanceData, CameraUniform, IndirectDrawCommand};
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    /// Global GPU type registry - single source of truth for all GPU types
    pub static ref GPU_REGISTRY: Mutex<UnifiedGpuSystem> = {
        let mut system = UnifiedGpuSystem::new();
        
        // Register all GPU types here
        // This is the ONLY place where GPU types are registered
        
        // Core terrain types
        system.register_type::<TerrainParamsSOA>();
        system.register_type::<BlockDistributionSOA>();
        
        // World storage types
        system.register_type::<ChunkMetadata>();
        system.register_type::<VoxelData>();
        
        // Rendering types - TODO: Implement AutoWgsl for these
        // system.register_type::<InstanceData>();
        // system.register_type::<CameraUniform>();
        // system.register_type::<IndirectDrawCommand>();
        
        Mutex::new(system)
    };
}

/// Initialize the GPU type registry (call once at startup)
pub fn initialize_gpu_registry() {
    // Force lazy initialization
    let _ = &*GPU_REGISTRY;
    
    // Validate all registered types
    let registry = GPU_REGISTRY.lock().unwrap();
    match registry.validate_all() {
        Ok(()) => log::info!("GPU type registry initialized successfully"),
        Err(errors) => {
            for error in errors {
                log::error!("GPU type validation error: {}", error);
            }
            panic!("GPU type validation failed");
        }
    }
}

/// Generate all WGSL type definitions
pub fn generate_all_gpu_types() -> String {
    let registry = GPU_REGISTRY.lock().unwrap();
    registry.generate_all_wgsl()
}

/// Generate shader bindings for a specific shader
pub fn generate_shader_bindings(shader_name: &str) -> String {
    let registry = GPU_REGISTRY.lock().unwrap();
    registry.generate_shader_bindings(shader_name)
}

/// Generate memory layout constants
pub fn generate_gpu_constants() -> String {
    let registry = GPU_REGISTRY.lock().unwrap();
    registry.generate_layout_constants()
}

/// Create a shader with automatic type and binding management
pub fn create_gpu_shader(
    device: &wgpu::Device,
    name: &str,
    shader_code: &str,
) -> Result<crate::gpu::automation::safe_pipeline::ValidatedShader, crate::gpu::automation::safe_pipeline::PipelineError> {
    let mut registry = GPU_REGISTRY.lock().unwrap();
    registry.create_shader(device, name, shader_code)
}