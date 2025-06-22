//! Macros for simplified bind group creation
//!
//! These macros reduce boilerplate for common bind group patterns while maintaining
//! type safety and clarity.

/// Create a bind group with minimal boilerplate
///
/// # Example
/// ```rust
/// let bind_group = create_bind_group!(
///     device,
///     "My Bind Group",
///     layout,
///     0 => world_buffer.as_entire_binding(),
///     1 => query_buffer.as_entire_binding(),
///     2 => result_buffer.as_entire_binding()
/// );
/// ```
#[macro_export]
macro_rules! create_bind_group {
    ($device:expr, $label:expr, $layout:expr, $($binding:expr => $resource:expr),+ $(,)?) => {
        $device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some($label),
            layout: $layout,
            entries: &[
                $(
                    wgpu::BindGroupEntry {
                        binding: $binding,
                        resource: $resource,
                    },
                )+
            ],
        })
    };
}

/// Create a bind group layout with minimal boilerplate
///
/// # Example
/// ```rust
/// let layout = create_bind_group_layout!(
///     device,
///     "My Layout",
///     0 => buffer(storage_read),
///     1 => buffer(storage),
///     2 => texture(2d),
///     3 => sampler(filtering)
/// );
/// ```
#[macro_export]
macro_rules! create_bind_group_layout {
    ($device:expr, $label:expr, $($binding:expr => $resource_type:ident($($args:tt)*)),+ $(,)?) => {
        $device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some($label),
            entries: &[
                $(
                    wgpu::BindGroupLayoutEntry {
                        binding: $binding,
                        visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: $crate::create_bind_group_layout!(@resource_type $resource_type($($args)*)),
                        count: None,
                    },
                )+
            ],
        })
    };

    // Resource type patterns
    (@resource_type buffer(storage)) => {
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    };

    (@resource_type buffer(storage_read)) => {
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    };

    (@resource_type buffer(uniform)) => {
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    };

    (@resource_type texture(2d)) => {
        wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        }
    };

    (@resource_type texture(3d)) => {
        wgpu::BindingType::Texture {
            multisampled: false,
            view_dimension: wgpu::TextureViewDimension::D3,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        }
    };

    (@resource_type sampler(filtering)) => {
        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
    };

    (@resource_type sampler(non_filtering)) => {
        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering)
    };
}

/// Define a struct that automatically implements bind group creation
///
/// # Example
/// ```rust
/// define_bind_group_data! {
///     struct TerrainBindings {
///         #[binding(0, storage_read)]
///         world_buffer: &WorldBuffer,
///         #[binding(1, storage)]
///         output_buffer: &Buffer,
///         #[binding(2, uniform)]
///         params: &Buffer,
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_bind_group_data {
    (
        $(#[$struct_meta:meta])*
        struct $name:ident {
            $(
                #[binding($binding:expr, $ty:ident)]
                $field:ident: $field_ty:ty
            ),+ $(,)?
        }
    ) => {
        $(#[$struct_meta])*
        pub struct $name {
            $(pub $field: $field_ty,)+
        }

        impl $name {
            /// Create the bind group layout for this binding set
            pub fn create_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
                $crate::create_bind_group_layout!(
                    device,
                    concat!(stringify!($name), " Layout"),
                    $($binding => buffer($ty)),+
                )
            }

            /// Create a bind group from this data
            pub fn create_bind_group(
                &self,
                device: &wgpu::Device,
                layout: &wgpu::BindGroupLayout,
            ) -> wgpu::BindGroup {
                $crate::create_bind_group!(
                    device,
                    concat!(stringify!($name), " Bind Group"),
                    layout,
                    $($binding => self.$field.as_entire_binding()),+
                )
            }
        }
    };
}

/// Helper for creating compute pipelines with automatic bind group layouts
///
/// # Example
/// ```rust
/// let pipeline = create_compute_pipeline!(
///     device,
///     "My Pipeline",
///     include_wgsl!("shader.wgsl"),
///     bind_groups: [
///         TerrainBindings::create_layout(&device),
///         LightingBindings::create_layout(&device),
///     ]
/// );
/// ```
#[macro_export]
macro_rules! create_compute_pipeline {
    (
        $device:expr,
        $label:expr,
        $shader_source:expr,
        bind_groups: [$($bind_group_layout:expr),* $(,)?]
    ) => {{
        let shader = $device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(concat!($label, " Shader")),
            source: wgpu::ShaderSource::Wgsl($shader_source.into()),
        });

        let pipeline_layout = $device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(concat!($label, " Layout")),
            bind_group_layouts: &[$($bind_group_layout,)*],
            push_constant_ranges: &[],
        });

        $device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some($label),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        })
    }};
}

/// Simplified buffer creation
///
/// # Example
/// ```rust
/// let buffer = create_buffer!(
///     device,
///     "My Buffer",
///     size: 1024,
///     usage: STORAGE | COPY_DST
/// );
/// ```
#[macro_export]
macro_rules! create_buffer {
    (
        $device:expr,
        $label:expr,
        size: $size:expr,
        usage: $($usage:ident)|+
    ) => {
        $device.create_buffer(&wgpu::BufferDescriptor {
            label: Some($label),
            size: $size,
            usage: wgpu::BufferUsages::$($usage)|+,
            mapped_at_creation: false,
        })
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macros_compile() {
        // This test just ensures the macros compile correctly
        // Real tests would require a GPU device
    }
}
