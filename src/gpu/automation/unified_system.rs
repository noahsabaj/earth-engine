//! Unified GPU type system - Single source of truth for all GPU operations
//! 
//! This module unifies all the automatic GPU systems into a cohesive whole,
//! where Rust types are the single source of truth for everything GPU-related.

use std::collections::HashMap;
use wgpu::{Device, ShaderModule, BindGroupLayout, PipelineLayout};
use crate::gpu::automation::{
    auto_wgsl::{AutoWgsl, WgslFieldMetadata},
    auto_layout::{AutoLayout, FieldOffset},
    auto_bindings::{AutoBindingLayout, BindingUsage},
    typed_bindings::BindingSlot,
    safe_pipeline::{PipelineError, ValidatedShader},
    shader_validator::{ShaderValidator, ValidationResult},
};

/// The unified GPU type registry - single source of truth
pub struct UnifiedGpuSystem {
    /// All registered GPU types
    types: HashMap<String, GpuTypeInfo>,
    /// All shader modules
    shaders: HashMap<String, ValidatedShader>,
    /// Binding layouts
    binding_layouts: HashMap<String, AutoBindingLayout>,
    /// Pipeline layouts
    pipeline_layouts: HashMap<String, PipelineLayout>,
}

/// Complete information about a GPU type
pub struct GpuTypeInfo {
    /// Rust type name
    pub rust_name: String,
    /// WGSL type name
    pub wgsl_name: String,
    /// WGSL definition
    pub wgsl_definition: String,
    /// Memory layout info
    pub layout: LayoutInfo,
    /// Binding slots where this type is used
    pub bindings: Vec<BindingSlotInfo>,
}

/// Layout information
pub struct LayoutInfo {
    pub size: u64,
    pub alignment: u64,
    pub stride: u64,
    pub fields: Vec<FieldOffset>,
}

/// Binding slot information
pub struct BindingSlotInfo {
    pub shader: String,
    pub group: u32,
    pub binding: u32,
    pub access: BindingAccess,
}

/// Binding access mode
#[derive(Debug, Clone, Copy)]
pub enum BindingAccess {
    ReadOnly,
    ReadWrite,
    Uniform,
}

impl UnifiedGpuSystem {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            shaders: HashMap::new(),
            binding_layouts: HashMap::new(),
            pipeline_layouts: HashMap::new(),
        }
    }
    
    /// Register a GPU type - this is the ONLY place types are defined
    pub fn register_type<T>(&mut self) 
    where
        T: AutoWgsl + AutoLayout + 'static,
    {
        let rust_name = std::any::type_name::<T>().to_string();
        let wgsl_name = T::wgsl_name().to_string();
        let wgsl_definition = T::generate_wgsl();
        
        let layout = LayoutInfo {
            size: T::gpu_size(),
            alignment: T::gpu_alignment(),
            stride: T::array_stride(),
            fields: T::field_offsets(),
        };
        
        let info = GpuTypeInfo {
            rust_name: rust_name.clone(),
            wgsl_name,
            wgsl_definition,
            layout,
            bindings: Vec::new(),
        };
        
        self.types.insert(rust_name, info);
    }
    
    /// Generate all WGSL type definitions
    pub fn generate_all_wgsl(&self) -> String {
        let mut wgsl = String::new();
        
        wgsl.push_str("// AUTO-GENERATED GPU TYPES - SINGLE SOURCE OF TRUTH\n");
        wgsl.push_str("// Generated from Rust type definitions\n\n");
        
        // Sort types by dependency order
        let sorted_types = self.topological_sort_types();
        
        for type_name in sorted_types {
            if let Some(info) = self.types.get(&type_name) {
                wgsl.push_str(&info.wgsl_definition);
                wgsl.push_str("\n\n");
            }
        }
        
        wgsl
    }
    
    /// Generate all binding declarations for a shader
    pub fn generate_shader_bindings(&self, shader_name: &str) -> String {
        let mut wgsl = String::new();
        
        wgsl.push_str(&format!("// Bindings for shader: {}\n", shader_name));
        
        // For now, generate standard bindings based on shader name
        // In the future, this should be driven by the type registry
        match shader_name {
            "terrain_generation_soa" => {
                // Add ChunkMetadata struct (not registered in unified system yet)
                wgsl.push_str("struct ChunkMetadata {\n");
                wgsl.push_str("    offset: vec3<i32>,\n");
                wgsl.push_str("    size: vec3<u32>,\n");
                wgsl.push_str("    voxel_count: u32,\n");
                wgsl.push_str("}\n\n");
                
                // Standard terrain generation bindings
                wgsl.push_str("@group(0) @binding(0) var<storage, read_write> world_data: array<u32>;\n");
                wgsl.push_str("@group(0) @binding(1) var<storage, read> metadata: array<ChunkMetadata>;\n");
                wgsl.push_str("@group(0) @binding(2) var<storage, read> params: TerrainParamsSOA;\n");
            }
            _ => {
                // Generic binding generation for other shaders
                let mut bindings: Vec<_> = self.types
                    .values()
                    .flat_map(|info| &info.bindings)
                    .filter(|binding| binding.shader == shader_name)
                    .collect();
                    
                // Sort by group then binding
                bindings.sort_by_key(|b| (b.group, b.binding));
                
                // Generate binding declarations
                for binding in bindings {
                    if let Some(type_info) = self.types.values().find(|t| {
                        t.bindings.iter().any(|b| b.group == binding.group && b.binding == binding.binding)
                    }) {
                        let access = match binding.access {
                            BindingAccess::ReadOnly => "<storage, read>",
                            BindingAccess::ReadWrite => "<storage, read_write>",
                            BindingAccess::Uniform => "<uniform>",
                        };
                        
                        wgsl.push_str(&format!(
                            "@group({}) @binding({}) var{} {}: {};\n",
                            binding.group,
                            binding.binding,
                            access,
                            type_info.wgsl_name.to_lowercase(),
                            type_info.wgsl_name
                        ));
                    }
                }
            }
        }
        
        wgsl
    }
    
    /// Create a complete shader with all required types and bindings
    pub fn create_shader(
        &mut self,
        device: &Device,
        name: &str,
        shader_code: &str,
    ) -> Result<ValidatedShader, PipelineError> {
        // Generate complete WGSL with types and bindings
        let mut complete_wgsl = String::new();
        
        // Add header comment
        complete_wgsl.push_str("// AUTO-GENERATED SHADER WITH UNIFIED GPU TYPES\n");
        complete_wgsl.push_str(&format!("// Shader: {}\n\n", name));
        
        // Add GPU constants first
        complete_wgsl.push_str("// GPU Constants\n");
        complete_wgsl.push_str("const CHUNK_SIZE: u32 = 32u;\n");
        complete_wgsl.push_str("const WORLD_SIZE: u32 = 512u;\n");
        complete_wgsl.push_str("const BLOCK_AIR: u32 = 0u;\n");
        complete_wgsl.push_str("const BLOCK_STONE: u32 = 1u;\n");
        complete_wgsl.push_str("const BLOCK_DIRT: u32 = 2u;\n");
        complete_wgsl.push_str("const BLOCK_GRASS: u32 = 3u;\n");
        complete_wgsl.push_str("\n");
        
        // Add all type definitions
        complete_wgsl.push_str(&self.generate_all_wgsl());
        complete_wgsl.push_str("\n");
        
        // Add bindings for this shader
        complete_wgsl.push_str(&self.generate_shader_bindings(name));
        complete_wgsl.push_str("\n");
        
        // Process includes in shader code
        let processed_shader = self.process_includes(shader_code);
        
        // Add the actual shader code
        complete_wgsl.push_str(&processed_shader);
        
        // Validate the complete shader
        let mut validator = ShaderValidator::new();
        match validator.validate_wgsl(name, &complete_wgsl) {
            ValidationResult::Ok => {},
            ValidationResult::Error(error) => {
                return Err(PipelineError::ShaderCompilation {
                    message: error.message,
                    source: complete_wgsl,
                });
            }
        }
        
        // Create shader module
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(name),
            source: wgpu::ShaderSource::Wgsl(complete_wgsl.into()),
        });
        
        // Extract metadata
        let entry_points = extract_entry_points(&processed_shader);
        let bindings = extract_bindings_from_system(self, name);
        
        let shader = ValidatedShader {
            module,
            entry_points,
            bindings,
        };
        
        Ok(shader)
    }
    
    /// Process #include directives
    fn process_includes(&self, shader_code: &str) -> String {
        let mut result = String::new();
        
        for line in shader_code.lines() {
            if line.trim().starts_with("#include") {
                // Skip includes - types come from unified system
                continue;
            }
            result.push_str(line);
            result.push('\n');
        }
        
        result
    }
    
    /// Get memory layout constants for all types
    pub fn generate_layout_constants(&self) -> String {
        let mut constants = String::new();
        
        constants.push_str("// AUTO-GENERATED MEMORY LAYOUT CONSTANTS\n\n");
        
        for (type_name, info) in &self.types {
            let prefix = info.wgsl_name.to_uppercase();
            
            constants.push_str(&format!("// {}\n", type_name));
            constants.push_str(&format!("pub const {}_SIZE: u64 = {};\n", prefix, info.layout.size));
            constants.push_str(&format!("pub const {}_ALIGNMENT: u64 = {};\n", prefix, info.layout.alignment));
            constants.push_str(&format!("pub const {}_STRIDE: u64 = {};\n", prefix, info.layout.stride));
            
            if !info.layout.fields.is_empty() {
                constants.push_str(&format!("\npub mod {}_offsets {{\n", info.wgsl_name.to_lowercase()));
                for field in &info.layout.fields {
                    constants.push_str(&format!(
                        "    pub const {}: u64 = {}; // {}\n",
                        field.name.to_uppercase(),
                        field.offset,
                        field.ty
                    ));
                }
                constants.push_str("}\n");
            }
            
            constants.push_str("\n");
        }
        
        constants
    }
    
    /// Validate that all types are correctly defined
    pub fn validate_all(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        
        // Check each type
        for (name, info) in &self.types {
            // Validate layout
            let mut last_offset = 0u64;
            for field in &info.layout.fields {
                if field.offset < last_offset {
                    errors.push(format!(
                        "Type {}: field {} overlaps previous field",
                        name, field.name
                    ));
                }
                last_offset = field.offset + field.size;
            }
            
            if last_offset > info.layout.size {
                errors.push(format!(
                    "Type {}: fields extend beyond type size",
                    name
                ));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Sort types by dependency order
    fn topological_sort_types(&self) -> Vec<String> {
        // For now, return in insertion order
        // TODO: Implement proper dependency resolution
        self.types.keys().cloned().collect()
    }
}

/// Extract entry points from shader code
fn extract_entry_points(shader_code: &str) -> Vec<String> {
    let mut entry_points = Vec::new();
    let re = regex::Regex::new(r"@(?:vertex|fragment|compute)\s+fn\s+(\w+)").unwrap();
    
    for capture in re.captures_iter(shader_code) {
        if let Some(name) = capture.get(1) {
            entry_points.push(name.as_str().to_string());
        }
    }
    
    entry_points
}

/// Extract bindings from the unified system
fn extract_bindings_from_system(
    system: &UnifiedGpuSystem,
    shader_name: &str,
) -> Vec<crate::gpu::automation::safe_pipeline::BindingMetadata> {
    let mut bindings = Vec::new();
    
    for type_info in system.types.values() {
        for binding in &type_info.bindings {
            if binding.shader == shader_name {
                bindings.push(crate::gpu::automation::safe_pipeline::BindingMetadata {
                    group: binding.group,
                    binding: binding.binding,
                    name: type_info.wgsl_name.clone(),
                    ty: type_info.wgsl_name.clone(),
                });
            }
        }
    }
    
    bindings
}

/// Macro to define a complete GPU type with everything automated
#[macro_export]
macro_rules! unified_gpu_type {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                pub $field:ident : $ty:ty
            ),* $(,)?
        }
    ) => {
        // Define the struct with all necessary derives
        $(#[$meta])*
        #[repr(C)]
        #[derive(
            Clone, Copy, Debug,
            encase::ShaderType,
            bytemuck::Pod,
            bytemuck::Zeroable,
        )]
        pub struct $name {
            $(
                $(#[$field_meta])*
                pub $field: $ty,
            )*
        }
        
        // Implement AutoWgsl
        $crate::auto_wgsl!(
            $name,
            name = stringify!($name),
            fields = [
                $( $field: $crate::gpu::automation::unified_system::wgsl_type_name::<$ty>() ),*
            ]
        );
        
        // Implement AutoLayout
        $crate::impl_auto_layout!(
            $name,
            fields = [
                $( $field : $ty = stringify!($field) ),*
            ]
        );
        
        // Implement unified type registration
        impl $name {
            /// Register this type in the unified GPU system
            pub fn register(system: &mut $crate::gpu::automation::unified_system::UnifiedGpuSystem) {
                system.register_type::<Self>();
            }
        }
    };
}

/// Get WGSL type name for a Rust type
pub fn wgsl_type_name<T>() -> &'static str {
    let type_name = std::any::type_name::<T>();
    
    match type_name {
        "u32" => "u32",
        "i32" => "i32", 
        "f32" => "f32",
        "[f32; 2]" => "vec2<f32>",
        "[f32; 3]" => "vec3<f32>",
        "[f32; 4]" => "vec4<f32>",
        "[u32; 2]" => "vec2<u32>",
        "[u32; 3]" => "vec3<u32>",
        "[u32; 4]" => "vec4<u32>",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Test the unified system
    unified_gpu_type! {
        /// Test vertex type
        pub struct UnifiedVertex {
            pub position: [f32; 3],
            pub normal: [f32; 3],
            pub uv: [f32; 2],
        }
    }
    
    #[test]
    fn test_unified_system() {
        let mut system = UnifiedGpuSystem::new();
        
        // Register the type
        UnifiedVertex::register(&mut system);
        
        // Generate WGSL
        let wgsl = system.generate_all_wgsl();
        assert!(wgsl.contains("struct UnifiedVertex"));
        
        // Generate constants
        let constants = system.generate_layout_constants();
        assert!(constants.contains("UNIFIEDVERTEX_SIZE"));
        
        // Validate
        assert!(system.validate_all().is_ok());
    }
}