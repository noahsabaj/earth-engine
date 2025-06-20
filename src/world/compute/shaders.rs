//! Shader management for unified compute system

use std::collections::HashMap;
use std::sync::Arc;

/// Manages compute shaders for the unified world system
pub struct ShaderManager {
    device: Arc<wgpu::Device>,
    shaders: HashMap<String, wgpu::ShaderModule>,
}

impl ShaderManager {
    /// Create a new shader manager
    pub fn new(device: Arc<wgpu::Device>) -> Result<Self, ShaderError> {
        let mut manager = Self {
            device,
            shaders: HashMap::new(),
        };
        
        // Load built-in shaders
        manager.load_builtin_shaders()?;
        
        Ok(manager)
    }
    
    /// Load all built-in compute shaders
    fn load_builtin_shaders(&mut self) -> Result<(), ShaderError> {
        let shaders = [
            ("terrain_generation", include_str!("../../shaders/compute/terrain_generation.wgsl")),
            ("chunk_modification", include_str!("../../shaders/compute/chunk_modification.wgsl")),
            ("unified_world_kernel", include_str!("../../shaders/compute/unified_world_kernel.wgsl")),
            ("hierarchical_physics", include_str!("../../shaders/compute/hierarchical_physics.wgsl")),
            ("ambient_occlusion", include_str!("../../shaders/compute/ambient_occlusion.wgsl")),
            ("weather_compute", include_str!("../../shaders/compute/weather_compute.wgsl")),
        ];
        
        for (name, source) in &shaders {
            self.load_shader(name, source)?;
        }
        
        Ok(())
    }
    
    /// Load a shader from source
    pub fn load_shader(&mut self, name: &str, source: &str) -> Result<(), ShaderError> {
        let module = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(name),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        
        self.shaders.insert(name.to_string(), module);
        Ok(())
    }
    
    /// Get a shader by name
    pub fn get_shader(&self, name: &str) -> Option<&wgpu::ShaderModule> {
        self.shaders.get(name)
    }
    
    /// List all available shaders
    pub fn list_shaders(&self) -> Vec<String> {
        self.shaders.keys().cloned().collect()
    }
}

/// Configuration for compute shaders
#[derive(Debug, Clone)]
pub struct ComputeShaderConfig {
    pub workgroup_size: [u32; 3],
    pub max_dispatches: u32,
    pub enable_debug: bool,
}

impl Default for ComputeShaderConfig {
    fn default() -> Self {
        Self {
            workgroup_size: [8, 8, 8],
            max_dispatches: 1000,
            enable_debug: false,
        }
    }
}

/// Shader system errors
#[derive(Debug, thiserror::Error)]
pub enum ShaderError {
    #[error("Shader compilation failed for {name}: {message}")]
    CompilationFailed { name: String, message: String },
    
    #[error("Shader not found: {name}")]
    ShaderNotFound { name: String },
    
    #[error("Invalid shader configuration: {field}")]
    InvalidConfig { field: String },
}