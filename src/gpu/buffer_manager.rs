//! Type-safe GPU buffer management

use std::sync::Arc;
use std::collections::HashMap;
use wgpu::util::DeviceExt;
use encase::{UniformBuffer, StorageBuffer, DynamicStorageBuffer, ShaderType, internal::WriteInto};
use crate::gpu::types::core::{GpuData, TypedGpuBuffer};
use crate::memory::BufferUsage;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GpuError {
    #[error("Buffer alignment error: {0}")]
    Alignment(String),
    
    #[error("Buffer size mismatch: expected {expected} bytes, got {actual} bytes")]
    SizeMismatch {
        expected: u64,
        actual: u64,
    },
    
    #[error("Buffer creation failed: {0}")]
    Creation(String),
    
    #[error("Buffer update failed: {0}")]
    Update(String),
}

/// Manages GPU buffers with automatic alignment and type safety
pub struct GpuBufferManager {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    
    /// Cache of buffer layouts for reuse
    layout_cache: HashMap<std::any::TypeId, wgpu::BindGroupLayout>,
}

impl GpuBufferManager {
    /// Create a new GPU buffer manager
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            device,
            queue,
            layout_cache: HashMap::new(),
        }
    }
    
    /// Create a uniform buffer with automatic alignment
    pub fn create_uniform<T: GpuData>(&self, data: &T) -> Result<TypedGpuBuffer<T>, GpuError> {
        // Use encase to handle alignment automatically
        let mut encoder = UniformBuffer::new(Vec::new());
        encoder.write(data)
            .map_err(|e| GpuError::Alignment(format!("Failed to encode uniform data: {}", e)))?;
        
        let bytes = encoder.into_inner();
        let size = bytes.len() as wgpu::BufferAddress;
        
        log::debug!(
            "[GpuBufferManager] Creating uniform buffer for {} ({} bytes)",
            std::any::type_name::<T>(),
            size
        );
        
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Uniform<{}>", std::any::type_name::<T>())),
            contents: &bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        Ok(TypedGpuBuffer::new(buffer, size))
    }
    
    /// Update an existing uniform buffer
    pub fn update_uniform<T: GpuData>(
        &self, 
        buffer: &TypedGpuBuffer<T>, 
        data: &T
    ) -> Result<(), GpuError> {
        let mut encoder = UniformBuffer::new(Vec::new());
        encoder.write(data)
            .map_err(|e| GpuError::Update(format!("Failed to encode uniform data: {}", e)))?;
        
        let bytes = encoder.into_inner();
        let new_size = bytes.len() as wgpu::BufferAddress;
        
        if new_size != buffer.size {
            return Err(GpuError::SizeMismatch {
                expected: buffer.size,
                actual: new_size,
            });
        }
        
        self.queue.write_buffer(&buffer.buffer, 0, &bytes);
        Ok(())
    }
    
    /// Create a storage buffer with automatic alignment
    pub fn create_storage<T: GpuData>(&self, data: &T) -> Result<TypedGpuBuffer<T>, GpuError> {
        let mut encoder = StorageBuffer::new(Vec::new());
        encoder.write(data)
            .map_err(|e| GpuError::Alignment(format!("Failed to encode storage data: {}", e)))?;
        
        let bytes = encoder.into_inner();
        let size = bytes.len() as wgpu::BufferAddress;
        
        log::debug!(
            "[GpuBufferManager] Creating storage buffer for {} ({} bytes)",
            std::any::type_name::<T>(),
            size
        );
        
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Storage<{}>", std::any::type_name::<T>())),
            contents: &bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        Ok(TypedGpuBuffer::new(buffer, size))
    }
    
    /// Create a storage buffer with a dynamic array
    pub fn create_storage_array<T: GpuData, E: GpuData>(
        &self,
        header: &T,
        elements: &[E],
    ) -> Result<TypedGpuBuffer<T>, GpuError> {
        let mut encoder = DynamicStorageBuffer::new(Vec::new());
        
        encoder.write(header)
            .map_err(|e| GpuError::Alignment(format!("Failed to encode header: {}", e)))?;
            
        encoder.write(elements)
            .map_err(|e| GpuError::Alignment(format!("Failed to encode array elements: {}", e)))?;
        
        let bytes = encoder.into_inner();
        let size = bytes.len() as wgpu::BufferAddress;
        
        log::debug!(
            "[GpuBufferManager] Creating storage array for {}+{}[{}] ({} bytes)",
            std::any::type_name::<T>(),
            std::any::type_name::<E>(),
            elements.len(),
            size
        );
        
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("StorageArray<{}>", std::any::type_name::<T>())),
            contents: &bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        Ok(TypedGpuBuffer::new(buffer, size))
    }
    
    /// Create a vertex buffer
    pub fn create_vertex<T: GpuData>(&self, data: &[T]) -> Result<TypedGpuBuffer<T>, GpuError> {
        let bytes = bytemuck::cast_slice(data);
        let size = bytes.len() as wgpu::BufferAddress;
        
        log::debug!(
            "[GpuBufferManager] Creating vertex buffer for {}[{}] ({} bytes)",
            std::any::type_name::<T>(),
            data.len(),
            size
        );
        
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Vertex<{}>", std::any::type_name::<T>())),
            contents: bytes,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        
        Ok(TypedGpuBuffer::new(buffer, size))
    }
    
    /// Update vertex buffer data
    pub fn update_vertex<T: GpuData>(
        &self, 
        buffer: &TypedGpuBuffer<T>, 
        data: &[T]
    ) -> Result<(), GpuError> {
        let bytes = bytemuck::cast_slice(data);
        let new_size = bytes.len() as wgpu::BufferAddress;
        
        if new_size > buffer.size {
            return Err(GpuError::SizeMismatch {
                expected: buffer.size,
                actual: new_size,
            });
        }
        
        self.queue.write_buffer(&buffer.buffer, 0, bytes);
        Ok(())
    }
    
    /// Get the device reference
    pub fn device(&self) -> &Arc<wgpu::Device> {
        &self.device
    }
    
    /// Get the queue reference
    pub fn queue(&self) -> &Arc<wgpu::Queue> {
        &self.queue
    }
}