//! Type-safe GPU binding system with automatic WGSL generation
//!
//! This module provides a type-safe binding system that automatically generates
//! binding indices and WGSL declarations from Rust types.

use crate::gpu::automation::auto_wgsl::AutoWgsl;
use crate::gpu::types::core::GpuData;
use std::marker::PhantomData;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BufferBinding, Device,
};

/// Type-safe binding slot
#[derive(Debug, Clone, Copy)]
pub struct BindingSlot<T: GpuData> {
    pub group: u32,
    pub binding: u32,
    _phantom: PhantomData<T>,
}

impl<T: GpuData> BindingSlot<T> {
    pub const fn new(group: u32, binding: u32) -> Self {
        Self {
            group,
            binding,
            _phantom: PhantomData,
        }
    }
}

/// Builder for type-safe bind groups
pub struct TypedBindGroupBuilder<'a> {
    device: &'a Device,
    layout: &'a BindGroupLayout,
    entries: Vec<BindGroupEntry<'a>>,
    label: Option<&'a str>,
}

impl<'a> TypedBindGroupBuilder<'a> {
    pub fn new(device: &'a Device, layout: &'a BindGroupLayout) -> Self {
        Self {
            device,
            layout,
            entries: Vec::new(),
            label: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Add a typed buffer binding
    pub fn buffer<T: GpuData>(
        mut self,
        slot: BindingSlot<T>,
        buffer: &'a wgpu::Buffer,
        offset: wgpu::BufferAddress,
        size: Option<wgpu::BufferAddress>,
    ) -> Self {
        self.entries.push(BindGroupEntry {
            binding: slot.binding,
            resource: wgpu::BindingResource::Buffer(BufferBinding {
                buffer,
                offset,
                size: size.and_then(std::num::NonZeroU64::new),
            }),
        });
        self
    }

    /// Build the bind group
    pub fn build(self) -> BindGroup {
        self.device.create_bind_group(&BindGroupDescriptor {
            label: self.label,
            layout: self.layout,
            entries: &self.entries,
        })
    }
}

/// Macro to define typed bindings with automatic indices
#[macro_export]
macro_rules! typed_bindings {
    (
        $vis:vis mod $mod_name:ident {
            group = $group:expr;
            $(
                $binding_name:ident: $type:ty = $binding_idx:expr;
            )*
        }
    ) => {
        $vis mod $mod_name {
            use super::*;
            use $crate::gpu::automation::typed_bindings::BindingSlot;

            $(
                pub const $binding_name: BindingSlot<$type> = BindingSlot::new($group, $binding_idx);
            )*

            /// Generate WGSL binding declarations
            pub fn generate_wgsl() -> String {
                let mut wgsl = String::new();
                wgsl.push_str(&format!("// Bind group {}\n", $group));

                $(
                    wgsl.push_str(&$crate::gpu::automation::typed_bindings::generate_binding_wgsl::<$type>(
                        $group,
                        $binding_idx,
                        stringify!($binding_name),
                    ));
                    wgsl.push('\n');
                )*

                wgsl
            }

            /// Get the bind group index
            pub const fn group() -> u32 {
                $group
            }
        }
    };
}

/// Generate WGSL binding declaration for a type
pub fn generate_binding_wgsl<T: GpuData + AutoWgsl>(
    group: u32,
    binding: u32,
    name: &str,
) -> String {
    format!(
        "@group({}) @binding({}) var<storage, read_write> {}: {};",
        group,
        binding,
        name.to_lowercase(),
        T::wgsl_name()
    )
}

/// Example usage of typed bindings
#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::buffer_layouts::CameraUniform;
    use crate::gpu::types::terrain::TerrainParams;

    typed_bindings! {
        pub mod render_bindings {
            group = 0;
            CAMERA: CameraUniform = 0;
            TERRAIN: TerrainParams = 1;
        }
    }

    #[test]
    fn test_typed_bindings() {
        assert_eq!(render_bindings::CAMERA.group, 0);
        assert_eq!(render_bindings::CAMERA.binding, 0);
        assert_eq!(render_bindings::TERRAIN.binding, 1);

        let wgsl = render_bindings::generate_wgsl();
        assert!(wgsl.contains("@group(0) @binding(0)"));
        assert!(wgsl.contains("@group(0) @binding(1)"));
    }
}
