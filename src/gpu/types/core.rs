//! Core GPU type system traits and utilities

use bytemuck::{Pod, Zeroable};
use encase::{internal::WriteInto, ShaderSize, ShaderType};
use std::marker::PhantomData;

/// Marker trait combining bytemuck and encase requirements for GPU data
///
/// This trait ensures that types can be:
/// - Safely cast to bytes (Pod + Zeroable)
/// - Properly aligned for GPU (ShaderType)
/// - Sent between threads (Send + Sync)
pub trait GpuData:
    ShaderType + ShaderSize + WriteInto + Pod + Zeroable + Send + Sync + 'static
{
}

/// Auto-implement GpuData for types that meet all requirements
impl<T> GpuData for T where
    T: ShaderType + ShaderSize + WriteInto + Pod + Zeroable + Send + Sync + 'static
{
}

/// Type-safe wrapper for GPU buffers
///
/// Ensures compile-time type safety and prevents buffer type mismatches
pub struct TypedGpuBuffer<T: GpuData> {
    /// The underlying WGPU buffer
    pub buffer: wgpu::Buffer,
    /// Size of the buffer in bytes
    pub size: wgpu::BufferAddress,
    /// Phantom data to maintain type information
    _phantom: PhantomData<T>,
}

impl<T: GpuData> TypedGpuBuffer<T> {
    /// Create a new typed GPU buffer
    pub fn new(buffer: wgpu::Buffer, size: wgpu::BufferAddress) -> Self {
        Self {
            buffer,
            size,
            _phantom: PhantomData,
        }
    }

    /// Get the buffer size
    pub fn size(&self) -> wgpu::BufferAddress {
        self.size
    }

    /// Get a reference to the underlying buffer
    pub fn raw(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

/// Common GPU vector types with proper alignment
#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone, Debug, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone, Debug, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone, Debug, Default)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

/// Validate GPU type alignment at compile time
#[macro_export]
macro_rules! validate_gpu_type {
    ($type:ty, $expected_size:expr) => {
        const _: () = {
            let size = std::mem::size_of::<$type>();
            assert!(
                size == $expected_size,
                concat!(
                    "GPU type ",
                    stringify!($type),
                    " has incorrect size. Expected ",
                    stringify!($expected_size),
                    " but got ",
                    stringify!(size)
                )
            );
        };
    };
}
