//! Type-safe GPU pipeline creation with compile-time validation
//! 
//! This module provides a type-safe pipeline creation system that eliminates
//! runtime panics and provides clear error messages at compile time.

use std::marker::PhantomData;
use wgpu::{Device, RenderPipeline, ComputePipeline, PipelineLayout};
use crate::error::EngineError;
use crate::gpu::automation::auto_wgsl::AutoWgsl;
use crate::gpu::types::core::GpuData;

/// Result type for pipeline operations
pub type PipelineResult<T> = Result<T, PipelineError>;

/// Enhanced pipeline creation errors
#[derive(Debug)]
pub enum PipelineError {
    ShaderCompilation { message: String, source: String },
    LayoutMismatch { expected: String, found: String },
    MissingBinding { group: u32, binding: u32, name: String },
    InvalidVertexAttribute { location: u32, expected: String },
    CreationFailed(String),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ShaderCompilation { message, .. } => {
                write!(f, "Shader compilation failed: {}", message)
            }
            Self::LayoutMismatch { expected, found } => {
                write!(f, "Pipeline layout mismatch: expected {}, found {}", expected, found)
            }
            Self::MissingBinding { group, binding, name } => {
                write!(f, "Missing required binding: group {}, binding {}, name: {}", group, binding, name)
            }
            Self::InvalidVertexAttribute { location, expected } => {
                write!(f, "Invalid vertex attribute: location {}, expected {}", location, expected)
            }
            Self::CreationFailed(msg) => {
                write!(f, "Pipeline creation failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for PipelineError {}

/// Type-safe render pipeline builder
pub struct TypedRenderPipelineBuilder<'a, V: GpuData> {
    device: &'a Device,
    label: Option<&'a str>,
    layout: Option<&'a PipelineLayout>,
    vertex_shader: Option<ValidatedShader>,
    fragment_shader: Option<ValidatedShader>,
    vertex_state: Option<wgpu::VertexState<'a>>,
    primitive: wgpu::PrimitiveState,
    depth_stencil: Option<wgpu::DepthStencilState>,
    multisample: wgpu::MultisampleState,
    targets: Vec<Option<wgpu::ColorTargetState>>,
    _phantom: PhantomData<V>,
}

/// Type-safe compute pipeline builder
pub struct TypedComputePipelineBuilder<'a> {
    device: &'a Device,
    label: Option<&'a str>,
    layout: Option<&'a PipelineLayout>,
    shader: Option<ValidatedShader>,
    entry_point: &'a str,
}

/// Validated shader module with metadata
pub struct ValidatedShader {
    pub module: wgpu::ShaderModule,
    pub entry_points: Vec<String>,
    pub bindings: Vec<BindingMetadata>,
}

/// Binding metadata for validation
#[derive(Debug, Clone)]
pub struct BindingMetadata {
    pub group: u32,
    pub binding: u32,
    pub name: String,
    pub ty: String,
}

impl<'a, V: GpuData> TypedRenderPipelineBuilder<'a, V> {
    pub fn new(device: &'a Device) -> Self {
        Self {
            device,
            label: None,
            layout: None,
            vertex_shader: None,
            fragment_shader: None,
            vertex_state: None,
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            targets: vec![],
            _phantom: PhantomData,
        }
    }
    
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
    
    pub fn layout(mut self, layout: &'a PipelineLayout) -> Self {
        self.layout = Some(layout);
        self
    }
    
    pub fn vertex_shader(mut self, shader: ValidatedShader) -> Self {
        self.vertex_shader = Some(shader);
        self
    }
    
    pub fn fragment_shader(mut self, shader: ValidatedShader) -> Self {
        self.fragment_shader = Some(shader);
        self
    }
    
    pub fn vertex_state(mut self, state: wgpu::VertexState<'a>) -> Self {
        self.vertex_state = Some(state);
        self
    }
    
    pub fn primitive(mut self, primitive: wgpu::PrimitiveState) -> Self {
        self.primitive = primitive;
        self
    }
    
    pub fn depth_stencil(mut self, state: wgpu::DepthStencilState) -> Self {
        self.depth_stencil = Some(state);
        self
    }
    
    pub fn targets(mut self, targets: Vec<Option<wgpu::ColorTargetState>>) -> Self {
        self.targets = targets;
        self
    }
    
    /// Build the pipeline with validation
    pub fn build(mut self) -> PipelineResult<RenderPipeline> {
        // Validate required fields
        let vertex_shader = self.vertex_shader
            .take()
            .ok_or_else(|| PipelineError::CreationFailed("Missing vertex shader".to_string()))?;
        
        let fragment_shader = self.fragment_shader
            .take()
            .ok_or_else(|| PipelineError::CreationFailed("Missing fragment shader".to_string()))?;
        
        let layout = self.layout
            .ok_or_else(|| PipelineError::CreationFailed("Missing pipeline layout".to_string()))?;
        
        let vertex_state = self.vertex_state
            .take()
            .ok_or_else(|| PipelineError::CreationFailed("Missing vertex state".to_string()))?;
        
        // Validate shader compatibility
        Self::validate_shader_bindings(&vertex_shader, &fragment_shader)?;
        
        // Create pipeline descriptor
        let descriptor = wgpu::RenderPipelineDescriptor {
            label: self.label,
            layout: Some(layout),
            vertex: vertex_state,
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader.module,
                entry_point: "fs_main",
                targets: &self.targets,
            }),
            primitive: self.primitive,
            depth_stencil: self.depth_stencil,
            multisample: self.multisample,
            multiview: None,
        };
        
        // Create pipeline with error handling
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.device.create_render_pipeline(&descriptor)
        })) {
            Ok(pipeline) => Ok(pipeline),
            Err(_) => Err(PipelineError::CreationFailed(
                "Pipeline creation panicked - check shader/layout compatibility".to_string()
            )),
        }
    }
    
    /// Validate that shader bindings match
    fn validate_shader_bindings(
        vertex: &ValidatedShader,
        fragment: &ValidatedShader,
    ) -> PipelineResult<()> {
        // Check for conflicting bindings
        for v_binding in &vertex.bindings {
            for f_binding in &fragment.bindings {
                if v_binding.group == f_binding.group && 
                   v_binding.binding == f_binding.binding &&
                   v_binding.ty != f_binding.ty {
                    return Err(PipelineError::LayoutMismatch {
                        expected: v_binding.ty.clone(),
                        found: f_binding.ty.clone(),
                    });
                }
            }
        }
        
        Ok(())
    }
}

impl<'a> TypedComputePipelineBuilder<'a> {
    pub fn new(device: &'a Device) -> Self {
        Self {
            device,
            label: None,
            layout: None,
            shader: None,
            entry_point: "main",
        }
    }
    
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
    
    pub fn layout(mut self, layout: &'a PipelineLayout) -> Self {
        self.layout = Some(layout);
        self
    }
    
    pub fn shader(mut self, shader: ValidatedShader) -> Self {
        self.shader = Some(shader);
        self
    }
    
    pub fn entry_point(mut self, entry_point: &'a str) -> Self {
        self.entry_point = entry_point;
        self
    }
    
    /// Build the compute pipeline with validation
    pub fn build(self) -> PipelineResult<ComputePipeline> {
        let shader = self.shader
            .ok_or_else(|| PipelineError::CreationFailed("Missing compute shader".to_string()))?;
        
        let layout = self.layout
            .ok_or_else(|| PipelineError::CreationFailed("Missing pipeline layout".to_string()))?;
        
        // Validate entry point exists
        if !shader.entry_points.contains(&self.entry_point.to_string()) {
            return Err(PipelineError::CreationFailed(
                format!("Entry point '{}' not found in shader", self.entry_point)
            ));
        }
        
        let descriptor = wgpu::ComputePipelineDescriptor {
            label: self.label,
            layout: Some(layout),
            module: &shader.module,
            entry_point: self.entry_point,
        };
        
        // Create pipeline with error handling
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.device.create_compute_pipeline(&descriptor)
        })) {
            Ok(pipeline) => Ok(pipeline),
            Err(_) => Err(PipelineError::CreationFailed(
                "Compute pipeline creation panicked - check shader/layout compatibility".to_string()
            )),
        }
    }
}

/// Validate and create a shader module
pub fn create_validated_shader(
    device: &Device,
    label: Option<&str>,
    source: &str,
) -> PipelineResult<ValidatedShader> {
    use crate::gpu::automation::shader_validator::{ShaderValidator, ValidationResult};
    
    // Validate shader first
    let mut validator = ShaderValidator::new();
    match validator.validate_wgsl(label.unwrap_or("shader"), source) {
        ValidationResult::Ok => {},
        ValidationResult::Error(error) => {
            return Err(PipelineError::ShaderCompilation {
                message: error.message,
                source: source.to_string(),
            });
        }
    }
    
    // Create shader module
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label,
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    
    // Extract metadata (simplified - in production, use naga for full parsing)
    let entry_points = extract_entry_points(source);
    let bindings = extract_bindings(source);
    
    Ok(ValidatedShader {
        module,
        entry_points,
        bindings,
    })
}

/// Extract entry points from WGSL source
fn extract_entry_points(source: &str) -> Vec<String> {
    let mut entry_points = Vec::new();
    
    // Simple regex for entry points
    let re = regex::Regex::new(r"@(vertex|fragment|compute)\s+fn\s+(\w+)")
        .expect("[SafePipeline] Failed to compile regex for entry point extraction");
    for capture in re.captures_iter(source) {
        if let Some(name) = capture.get(2) {
            entry_points.push(name.as_str().to_string());
        }
    }
    
    entry_points
}

/// Extract binding information from WGSL source
fn extract_bindings(source: &str) -> Vec<BindingMetadata> {
    let mut bindings = Vec::new();
    
    // Simple regex for bindings
    let re = regex::Regex::new(
        r"@group\((\d+)\)\s*@binding\((\d+)\)\s*var(?:<[^>]+>)?\s+(\w+)\s*:\s*([^;]+)"
    ).expect("[SafePipeline] Failed to compile regex for binding extraction");
    
    for capture in re.captures_iter(source) {
        if let (Some(group), Some(binding), Some(name), Some(ty)) = 
            (capture.get(1), capture.get(2), capture.get(3), capture.get(4)) {
            bindings.push(BindingMetadata {
                group: group.as_str().parse().unwrap_or(0),
                binding: binding.as_str().parse().unwrap_or(0),
                name: name.as_str().to_string(),
                ty: ty.as_str().trim().to_string(),
            });
        }
    }
    
    bindings
}

/// Macro for creating type-safe pipelines
#[macro_export]
macro_rules! create_typed_pipeline {
    (
        render $device:expr,
        vertex_type = $vertex_type:ty,
        vertex_shader = $vs_source:expr,
        fragment_shader = $fs_source:expr,
        layout = $layout:expr
        $(, $option:ident = $value:expr)*
    ) => {{
        use $crate::gpu::safe_pipeline::{TypedRenderPipelineBuilder, create_validated_shader};
        
        let vs = create_validated_shader($device, Some("vertex shader"), $vs_source)?;
        let fs = create_validated_shader($device, Some("fragment shader"), $fs_source)?;
        
        TypedRenderPipelineBuilder::<$vertex_type>::new($device)
            .vertex_shader(vs)
            .fragment_shader(fs)
            .layout($layout)
            $( .$option($value) )*
            .build()
    }};
    
    (
        compute $device:expr,
        shader = $source:expr,
        layout = $layout:expr
        $(, $option:ident = $value:expr)*
    ) => {{
        use $crate::gpu::safe_pipeline::{TypedComputePipelineBuilder, create_validated_shader};
        
        let shader = create_validated_shader($device, Some("compute shader"), $source)?;
        
        TypedComputePipelineBuilder::new($device)
            .shader(shader)
            .layout($layout)
            $( .$option($value) )*
            .build()
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_entry_point_extraction() {
        let source = r#"
            @vertex
            fn vs_main() -> @builtin(position) vec4<f32> {
                return vec4<f32>(0.0, 0.0, 0.0, 1.0);
            }
            
            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return vec4<f32>(1.0, 0.0, 0.0, 1.0);
            }
        "#;
        
        let entry_points = extract_entry_points(source);
        assert_eq!(entry_points.len(), 2);
        assert!(entry_points.contains(&"vs_main".to_string()));
        assert!(entry_points.contains(&"fs_main".to_string()));
    }
    
    #[test]
    fn test_binding_extraction() {
        let source = r#"
            @group(0) @binding(0) var<uniform> camera: CameraUniform;
            @group(0) @binding(1) var<storage, read> instances: array<Instance>;
        "#;
        
        let bindings = extract_bindings(source);
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].name, "camera");
        assert_eq!(bindings[0].group, 0);
        assert_eq!(bindings[0].binding, 0);
    }
}