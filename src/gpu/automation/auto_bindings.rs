//! Automatic GPU binding generation from usage
//!
//! This module provides automatic binding index assignment based on shader usage,
//! eliminating the need for manual binding management.

use std::collections::{HashMap, HashSet};
use wgpu::{BindingType, BufferBindingType, ShaderStages};

/// Binding usage information extracted from shaders
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BindingUsage {
    pub name: String,
    pub group: u32,
    pub ty: BindingTypeInfo,
    pub stages: ShaderStages,
}

/// Simplified binding type information
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BindingTypeInfo {
    UniformBuffer,
    StorageBuffer { read_only: bool },
    Texture2D,
    Sampler,
}

/// Automatic binding layout generator
pub struct AutoBindingLayout {
    /// Map from (group, name) to binding index
    binding_indices: HashMap<(u32, String), u32>,
    /// All binding usages
    usages: Vec<BindingUsage>,
}

impl AutoBindingLayout {
    pub fn new() -> Self {
        Self {
            binding_indices: HashMap::new(),
            usages: Vec::new(),
        }
    }

    /// Extract bindings from WGSL source
    pub fn extract_from_wgsl(&mut self, wgsl: &str) {
        // Simple regex-based extraction (in production, use naga for proper parsing)
        let binding_regex = regex::Regex::new(
            r"@group\((\d+)\)\s*@binding\((\d+)\)\s*var(?:<([^>]+)>)?\s+(\w+)\s*:\s*([^;]+);",
        )
        .expect("[AutoBindings] Failed to compile regex for binding extraction");

        for capture in binding_regex.captures_iter(wgsl) {
            let group = capture[1]
                .parse::<u32>()
                .expect("[AutoBindings] Failed to parse group number from regex capture");
            let _binding = capture[2]
                .parse::<u32>()
                .expect("[AutoBindings] Failed to parse binding number from regex capture"); // Ignore manual binding
            let storage_type = capture.get(3).map(|m| m.as_str());
            let name = capture[4].to_string();
            let type_str = capture[5].trim();

            let ty = match storage_type {
                Some("storage, read") => BindingTypeInfo::StorageBuffer { read_only: true },
                Some("storage, read_write") | Some("storage") => {
                    BindingTypeInfo::StorageBuffer { read_only: false }
                }
                None if type_str.contains("sampler") => BindingTypeInfo::Sampler,
                None if type_str.contains("texture") => BindingTypeInfo::Texture2D,
                None => BindingTypeInfo::UniformBuffer,
                _ => BindingTypeInfo::StorageBuffer { read_only: true },
            };

            self.add_usage(BindingUsage {
                name: name.clone(),
                group,
                ty,
                stages: ShaderStages::all(), // Would be determined by shader type
            });
        }
    }

    /// Add a binding usage
    pub fn add_usage(&mut self, usage: BindingUsage) {
        let key = (usage.group, usage.name.clone());

        // Assign binding index if not already assigned
        if !self.binding_indices.contains_key(&key) {
            let group_bindings = self
                .binding_indices
                .iter()
                .filter(|((g, _), _)| *g == usage.group)
                .count() as u32;

            self.binding_indices.insert(key, group_bindings);
        }

        self.usages.push(usage);
    }

    /// Get binding index for a named binding
    pub fn get_binding(&self, group: u32, name: &str) -> Option<u32> {
        self.binding_indices
            .get(&(group, name.to_string()))
            .copied()
    }

    /// Generate bind group layout entries
    pub fn generate_layout_entries(&self, group: u32) -> Vec<wgpu::BindGroupLayoutEntry> {
        let mut entries = Vec::new();
        let mut seen = HashSet::new();

        for usage in &self.usages {
            if usage.group != group {
                continue;
            }

            let binding = self.binding_indices[&(usage.group, usage.name.clone())];

            // Skip duplicates
            if !seen.insert(binding) {
                continue;
            }

            let ty = match &usage.ty {
                BindingTypeInfo::UniformBuffer => BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                BindingTypeInfo::StorageBuffer { read_only } => BindingType::Buffer {
                    ty: BufferBindingType::Storage {
                        read_only: *read_only,
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                BindingTypeInfo::Texture2D => BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                BindingTypeInfo::Sampler => {
                    BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
                }
            };

            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: usage.stages,
                ty,
                count: None,
            });
        }

        // Sort by binding index
        entries.sort_by_key(|e| e.binding);
        entries
    }

    /// Generate WGSL binding declarations with automatic indices
    pub fn generate_wgsl_bindings(&self, group: u32) -> String {
        let mut wgsl = String::new();
        let mut processed = HashSet::new();

        wgsl.push_str(&format!("// Auto-generated bindings for group {}\n", group));

        for usage in &self.usages {
            if usage.group != group {
                continue;
            }

            let binding = self.binding_indices[&(usage.group, usage.name.clone())];

            // Skip duplicates
            if !processed.insert((binding, &usage.name)) {
                continue;
            }

            let var_decl = match &usage.ty {
                BindingTypeInfo::UniformBuffer => {
                    format!(
                        "var<uniform> {}: {}",
                        usage.name,
                        infer_type_from_name(&usage.name)
                    )
                }
                BindingTypeInfo::StorageBuffer { read_only: true } => {
                    format!(
                        "var<storage, read> {}: {}",
                        usage.name,
                        infer_type_from_name(&usage.name)
                    )
                }
                BindingTypeInfo::StorageBuffer { read_only: false } => {
                    format!(
                        "var<storage, read_write> {}: {}",
                        usage.name,
                        infer_type_from_name(&usage.name)
                    )
                }
                BindingTypeInfo::Texture2D => {
                    format!("var {}: texture_2d<f32>", usage.name)
                }
                BindingTypeInfo::Sampler => {
                    format!("var {}: sampler", usage.name)
                }
            };

            wgsl.push_str(&format!(
                "@group({}) @binding({}) {};\n",
                group, binding, var_decl
            ));
        }

        wgsl
    }
}

/// Infer WGSL type from binding name
fn infer_type_from_name(name: &str) -> &'static str {
    let lower = name.to_lowercase();

    if lower.contains("camera") {
        "CameraUniform"
    } else if lower.contains("instance") {
        "array<InstanceData>"
    } else if lower.contains("world") || lower.contains("voxel") {
        "array<u32>"
    } else if lower.contains("terrain") {
        "TerrainParamsSOA"
    } else if lower.contains("metadata") {
        "array<ChunkMetadata>"
    } else {
        "array<vec4<f32>>"
    }
}

/// Macro to define shader bindings with automatic index assignment
#[macro_export]
macro_rules! shader_bindings {
    (
        $shader_name:ident {
            $(
                group($group:expr) {
                    $(
                        $name:ident: $ty:ident $(<$access:ident>)? in $stages:expr
                    ),* $(,)?
                }
            )*
        }
    ) => {
        pub mod $shader_name {
            use super::*;
            use $crate::gpu::automation::auto_bindings::{AutoBindingLayout, BindingUsage, BindingTypeInfo};
            use wgpu::ShaderStages;

            lazy_static::lazy_static! {
                static ref LAYOUT: AutoBindingLayout = {
                    let mut layout = AutoBindingLayout::new();

                    $(
                        $(
                            layout.add_usage(BindingUsage {
                                name: stringify!($name).to_string(),
                                group: $group,
                                ty: shader_bindings!(@binding_type $ty $($access)?),
                                stages: $stages,
                            });
                        )*
                    )*

                    layout
                };
            }

            $(
                paste::paste! {
                    pub mod [<group_ $group>] {
                        use super::*;

                        $(
                            pub fn $name() -> u32 {
                                LAYOUT.get_binding($group, stringify!($name))
                                    .expect(concat!("Binding ", stringify!($name), " not found"))
                            }
                        )*

                        pub fn layout_entries() -> Vec<wgpu::BindGroupLayoutEntry> {
                            LAYOUT.generate_layout_entries($group)
                        }

                        pub fn wgsl_bindings() -> String {
                            LAYOUT.generate_wgsl_bindings($group)
                        }
                    }
                }
            )*
        }
    };

    (@binding_type uniform) => { BindingTypeInfo::UniformBuffer };
    (@binding_type storage) => { BindingTypeInfo::StorageBuffer { read_only: false } };
    (@binding_type storage read) => { BindingTypeInfo::StorageBuffer { read_only: true } };
    (@binding_type texture) => { BindingTypeInfo::Texture2D };
    (@binding_type sampler) => { BindingTypeInfo::Sampler };
}

#[cfg(test)]
mod tests {
    use super::*;

    shader_bindings! {
        terrain_shader {
            group(0) {
                camera: uniform in ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                world_data: storage in ShaderStages::COMPUTE,
                metadata: storage<read> in ShaderStages::COMPUTE,
                params: storage<read> in ShaderStages::COMPUTE,
            }
            group(1) {
                texture_atlas: texture in ShaderStages::FRAGMENT,
                atlas_sampler: sampler in ShaderStages::FRAGMENT,
            }
        }
    }

    #[test]
    fn test_auto_bindings() {
        // Check that bindings are assigned sequentially
        assert_eq!(terrain_shader::group_0::camera(), 0);
        assert_eq!(terrain_shader::group_0::world_data(), 1);
        assert_eq!(terrain_shader::group_0::metadata(), 2);
        assert_eq!(terrain_shader::group_0::params(), 3);

        assert_eq!(terrain_shader::group_1::texture_atlas(), 0);
        assert_eq!(terrain_shader::group_1::atlas_sampler(), 1);

        // Check WGSL generation
        let wgsl = terrain_shader::group_0::wgsl_bindings();
        assert!(wgsl.contains("@group(0) @binding(0) var<uniform> camera"));
        assert!(wgsl.contains("@group(0) @binding(1) var<storage, read_write> world_data"));
    }
}
