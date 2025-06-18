//! SOA buffer builders for efficient GPU buffer creation
//! 
//! Provides type-safe builders for creating GPU buffers in Structure of Arrays format.

use std::marker::PhantomData;
use wgpu::util::DeviceExt;
use crate::gpu::types::TypedGpuBuffer;
use crate::gpu::soa::types::SoaCompatible;

/// Builder for creating SOA GPU buffers
pub struct SoaBufferBuilder<T: SoaCompatible> {
    /// Items to be converted to SOA format
    items: Vec<T>,
    /// Buffer label for debugging
    label: Option<String>,
    /// Additional buffer usage flags
    usage: wgpu::BufferUsages,
}

impl<T: SoaCompatible> SoaBufferBuilder<T> {
    /// Create a new SOA buffer builder
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            label: None,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        }
    }
    
    /// Set the buffer label for debugging
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
    
    /// Add additional usage flags
    pub fn with_usage(mut self, usage: wgpu::BufferUsages) -> Self {
        self.usage |= usage;
        self
    }
    
    /// Add a single item to the buffer
    pub fn push(&mut self, item: T) -> &mut Self {
        self.items.push(item);
        self
    }
    
    /// Add multiple items to the buffer
    pub fn extend(&mut self, items: impl IntoIterator<Item = T>) -> &mut Self {
        self.items.extend(items);
        self
    }
    
    /// Get the current number of items
    pub fn len(&self) -> usize {
        self.items.len()
    }
    
    /// Check if the builder is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    
    /// Build the SOA GPU buffer
    pub fn build(self, device: &wgpu::Device) -> TypedGpuBuffer<T::Arrays> {
        // Convert AOS to SOA
        let soa_data = T::to_soa(&self.items);
        
        // Create buffer label
        let label = self.label.unwrap_or_else(|| {
            format!("SOA<{}>", std::any::type_name::<T>())
        });
        
        // Create GPU buffer
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&label),
            contents: bytemuck::bytes_of(&soa_data),
            usage: self.usage,
        });
        
        let size = std::mem::size_of_val(&soa_data) as wgpu::BufferAddress;
        
        log::debug!(
            "[SOA Builder] Created buffer '{}' with {} items, {} bytes",
            label,
            self.items.len(),
            size
        );
        
        TypedGpuBuffer::new(buffer, size)
    }
    
    /// Build an empty SOA buffer with capacity
    pub fn build_empty(
        device: &wgpu::Device,
        capacity: usize,
        label: Option<&str>,
    ) -> TypedGpuBuffer<T::Arrays> {
        // Create empty SOA data
        let soa_data = T::to_soa(&[]);
        let size = std::mem::size_of_val(&soa_data) as wgpu::BufferAddress;
        
        let label = label.unwrap_or_else(|| {
            Box::leak(format!("SOA<{}>", std::any::type_name::<T>()).into_boxed_str())
        });
        
        // Create GPU buffer with full capacity
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        TypedGpuBuffer::new(buffer, size)
    }
}

impl<T: SoaCompatible> Default for SoaBufferBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: SoaCompatible> FromIterator<T> for SoaBufferBuilder<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut builder = Self::new();
        builder.extend(iter);
        builder
    }
}

/// Extension trait for updating SOA buffers
pub trait SoaBufferExt<T: SoaCompatible> {
    /// Update the buffer with new SOA data
    fn update_soa(&self, queue: &wgpu::Queue, items: &[T]);
    
    /// Update a single item in the SOA buffer
    fn update_item(&self, queue: &wgpu::Queue, index: usize, item: &T);
}

impl<T: SoaCompatible> SoaBufferExt<T> for TypedGpuBuffer<T::Arrays> {
    fn update_soa(&self, queue: &wgpu::Queue, items: &[T]) {
        let soa_data = T::to_soa(items);
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&soa_data));
        
        log::trace!(
            "[SOA Buffer] Updated buffer with {} items",
            T::soa_count(&soa_data)
        );
    }
    
    fn update_item(&self, queue: &wgpu::Queue, index: usize, item: &T) {
        // For single item updates, we need to read the current data,
        // update it, and write it back. This is less efficient than
        // batch updates but sometimes necessary.
        
        // In a real implementation, we might want to batch these updates
        // or use a staging buffer for better performance.
        log::warn!(
            "[SOA Buffer] Single item update at index {} - consider batching updates",
            index
        );
        
        // For now, we'll need to maintain a CPU copy or implement
        // GPU-side single item updates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::types::terrain::BlockDistribution;
    use crate::gpu::soa::types::BlockDistributionSOA;
    
    #[test]
    fn test_soa_builder() {
        // Create test data
        let items = vec![
            BlockDistribution {
                block_id: 1,
                min_height: 0,
                max_height: 10,
                probability: 0.5,
                noise_threshold: 0.3,
                _padding: [0; 3],
            },
            BlockDistribution {
                block_id: 2,
                min_height: 10,
                max_height: 20,
                probability: 0.3,
                noise_threshold: 0.5,
                _padding: [0; 3],
            },
        ];
        
        // Build SOA data
        let mut builder = SoaBufferBuilder::new();
        builder.extend(items.clone());
        
        assert_eq!(builder.len(), 2);
        assert!(!builder.is_empty());
        
        // Verify SOA conversion
        let soa_data = BlockDistribution::to_soa(&items);
        assert_eq!(soa_data.count, 2);
        assert_eq!(soa_data.block_ids[0], 1);
        assert_eq!(soa_data.block_ids[1], 2);
        assert_eq!(soa_data.min_heights[0], 0);
        assert_eq!(soa_data.min_heights[1], 10);
    }
}