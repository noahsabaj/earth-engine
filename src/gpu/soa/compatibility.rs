//! Compatibility layer for gradual AOS to SOA migration
//! 
//! Provides a unified interface that can work with both Array of Structures (AOS)
//! and Structure of Arrays (SOA) representations during the migration period.

use std::marker::PhantomData;
use crate::gpu::types::TypedGpuBuffer;
use crate::gpu::soa::types::SoaCompatible;
use crate::gpu::buffer_manager::GpuError;

/// Unified GPU buffer that supports both AOS and SOA layouts
/// 
/// Note: For AOS mode, T must implement GpuData directly.
/// For SOA mode, T::Arrays must implement GpuData.
pub enum UnifiedGpuBuffer<T> {
    /// Traditional Array of Structures layout
    ArrayOfStructs {
        buffer: wgpu::Buffer,
        size: wgpu::BufferAddress,
        count: usize,
        _phantom: PhantomData<T>,
    },
    
    /// Optimized Structure of Arrays layout  
    StructureOfArrays {
        buffer: wgpu::Buffer,
        size: wgpu::BufferAddress,
        _phantom: PhantomData<T>,
    },
}

impl<T: SoaCompatible> UnifiedGpuBuffer<T> {
    /// Create a new AOS buffer (legacy)
    /// Requires T to implement GpuData
    pub fn new_aos(buffer: wgpu::Buffer, size: wgpu::BufferAddress, count: usize) -> Self {
        Self::ArrayOfStructs { 
            buffer, 
            size,
            count,
            _phantom: PhantomData,
        }
    }
    
    /// Create a new SOA buffer (optimized)
    /// Requires T::Arrays to implement GpuData
    pub fn new_soa(buffer: wgpu::Buffer, size: wgpu::BufferAddress) -> Self {
        Self::StructureOfArrays {
            buffer,
            size,
            _phantom: PhantomData,
        }
    }
    
    /// Create from TypedGpuBuffer<T> where T: GpuData
    pub fn from_aos_typed<U>(typed_buffer: TypedGpuBuffer<U>, count: usize) -> Self
    where
        U: crate::gpu::GpuData,
        T: From<U>,
    {
        Self::ArrayOfStructs {
            buffer: typed_buffer.buffer,
            size: typed_buffer.size,
            count,
            _phantom: PhantomData,
        }
    }
    
    /// Create from TypedGpuBuffer<T::Arrays>
    pub fn from_soa_typed(typed_buffer: TypedGpuBuffer<T::Arrays>) -> Self {
        Self::StructureOfArrays {
            buffer: typed_buffer.buffer,
            size: typed_buffer.size,
            _phantom: PhantomData,
        }
    }
    
    /// Check if this buffer uses SOA layout
    pub fn is_soa(&self) -> bool {
        matches!(self, Self::StructureOfArrays { .. })
    }
    
    /// Get the underlying wgpu buffer
    pub fn raw_buffer(&self) -> &wgpu::Buffer {
        match self {
            Self::ArrayOfStructs { buffer, .. } => buffer,
            Self::StructureOfArrays { buffer, .. } => buffer,
        }
    }
    
    /// Get buffer size in bytes
    pub fn size(&self) -> wgpu::BufferAddress {
        match self {
            Self::ArrayOfStructs { size, .. } => *size,
            Self::StructureOfArrays { size, .. } => *size,
        }
    }
    
    /// Update buffer with new data
    pub fn update(&self, queue: &wgpu::Queue, data: &[T]) -> Result<(), GpuError> {
        match self {
            Self::ArrayOfStructs { buffer, .. } => {
                // Direct update for AOS
                queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
                Ok(())
            }
            Self::StructureOfArrays { buffer, .. } => {
                // Convert to SOA and update
                let soa_data = T::to_soa(data);
                queue.write_buffer(buffer, 0, bytemuck::bytes_of(&soa_data));
                Ok(())
            }
        }
    }
}

/// Configuration for choosing buffer layout during migration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferLayoutPreference {
    /// Use traditional AOS layout
    ArrayOfStructs,
    /// Use optimized SOA layout
    StructureOfArrays,
    /// Automatically choose based on data characteristics
    Auto,
}

impl Default for BufferLayoutPreference {
    fn default() -> Self {
        Self::Auto
    }
}

/// Migration helper for transitioning systems to SOA
pub struct SoaMigrationHelper {
    /// Current layout preference
    preference: BufferLayoutPreference,
    /// Track migration progress
    migrated_systems: Vec<String>,
}

impl SoaMigrationHelper {
    /// Create a new migration helper
    pub fn new(preference: BufferLayoutPreference) -> Self {
        Self {
            preference,
            migrated_systems: Vec::new(),
        }
    }
    
    /// Decide which layout to use for a given system
    pub fn choose_layout<T: SoaCompatible>(
        &self,
        system_name: &str,
        item_count: usize,
    ) -> BufferLayoutPreference {
        match self.preference {
            BufferLayoutPreference::Auto => {
                // Heuristics for automatic choice
                if self.migrated_systems.contains(&system_name.to_string()) {
                    BufferLayoutPreference::StructureOfArrays
                } else if item_count > 64 {
                    // SOA benefits from larger arrays
                    BufferLayoutPreference::StructureOfArrays
                } else {
                    BufferLayoutPreference::ArrayOfStructs
                }
            }
            other => other,
        }
    }
    
    /// Mark a system as migrated to SOA
    pub fn mark_migrated(&mut self, system_name: impl Into<String>) {
        let name = system_name.into();
        if !self.migrated_systems.contains(&name) {
            self.migrated_systems.push(name);
            log::info!("[SOA Migration] System '{}' migrated to SOA", 
                      self.migrated_systems.last().unwrap());
        }
    }
    
    /// Get migration progress
    pub fn progress(&self) -> MigrationProgress {
        MigrationProgress {
            total_systems: 10, // Estimated total systems
            migrated_systems: self.migrated_systems.len(),
            migrated_list: self.migrated_systems.clone(),
        }
    }
}

/// Migration progress tracking
#[derive(Debug, Clone)]
pub struct MigrationProgress {
    pub total_systems: usize,
    pub migrated_systems: usize,
    pub migrated_list: Vec<String>,
}

impl MigrationProgress {
    /// Get completion percentage
    pub fn percentage(&self) -> f32 {
        if self.total_systems == 0 {
            0.0
        } else {
            (self.migrated_systems as f32 / self.total_systems as f32) * 100.0
        }
    }
}

/// Extension trait for gradual migration
pub trait GpuBufferMigrationExt {
    /// Create a unified buffer based on migration preferences
    fn create_unified_buffer<T: SoaCompatible>(
        &self,
        data: &[T],
        label: Option<&str>,
        preference: BufferLayoutPreference,
    ) -> Result<UnifiedGpuBuffer<T>, GpuError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_migration_helper() {
        let mut helper = SoaMigrationHelper::new(BufferLayoutPreference::Auto);
        
        // Test automatic choice
        assert_eq!(
            helper.choose_layout::<crate::gpu::BlockDistribution>("terrain", 100),
            BufferLayoutPreference::StructureOfArrays
        );
        
        // Mark as migrated
        helper.mark_migrated("terrain");
        
        // Check progress
        let progress = helper.progress();
        assert_eq!(progress.migrated_systems, 1);
        assert!(progress.migrated_list.contains(&"terrain".to_string()));
    }
}