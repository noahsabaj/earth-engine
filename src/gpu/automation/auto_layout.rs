//! Automatic GPU memory layout calculation
//! 
//! This module provides automatic calculation of memory layouts, offsets, 
//! and strides, eliminating all manual padding and alignment calculations.

use std::mem;
use encase::{ShaderType, ShaderSize};
use crate::gpu::types::core::GpuData;

/// Trait for types with automatic layout calculation
pub trait AutoLayout: GpuData {
    /// Get the size of this type in GPU memory
    fn gpu_size() -> u64 {
        Self::SHADER_SIZE.get()
    }
    
    /// Get the alignment requirement
    fn gpu_alignment() -> u64 {
        // WGSL requires 16-byte alignment for most types
        Self::min_size().get().max(16)
    }
    
    /// Calculate the stride for an array of this type
    fn array_stride() -> u64 {
        let size = Self::gpu_size();
        let alignment = Self::gpu_alignment();
        align_size(size, alignment)
    }
    
    /// Get field offsets (for types that support it)
    fn field_offsets() -> Vec<FieldOffset> {
        vec![]
    }
}

/// Field offset information
#[derive(Debug, Clone)]
pub struct FieldOffset {
    pub name: &'static str,
    pub offset: u64,
    pub size: u64,
    pub ty: String,
}

/// Align a size to the given alignment
pub fn align_size(size: u64, alignment: u64) -> u64 {
    (size + alignment - 1) & !(alignment - 1)
}

/// Calculate padding needed for alignment
pub fn padding_for_alignment(current_offset: u64, alignment: u64) -> u64 {
    (alignment - (current_offset % alignment)) % alignment
}

/// Memory layout builder for complex structures
pub struct LayoutBuilder {
    current_offset: u64,
    fields: Vec<FieldOffset>,
    total_size: u64,
}

impl LayoutBuilder {
    pub fn new() -> Self {
        Self {
            current_offset: 0,
            fields: Vec::new(),
            total_size: 0,
        }
    }
    
    /// Add a field with automatic padding
    pub fn add_field<T: ShaderType + ShaderSize>(
        &mut self,
        name: &'static str,
        ty_name: &'static str,
    ) -> &mut Self {
        let size = T::SHADER_SIZE.get();
        let alignment = T::min_size().get();
        
        // Add padding if needed
        let padding = padding_for_alignment(self.current_offset, alignment);
        self.current_offset += padding;
        
        // Add field
        self.fields.push(FieldOffset {
            name,
            offset: self.current_offset,
            size,
            ty: ty_name.to_string(),
        });
        
        self.current_offset += size;
        self.total_size = self.current_offset;
        
        self
    }
    
    /// Add an array field
    pub fn add_array<T: ShaderType + ShaderSize>(
        &mut self,
        name: &'static str,
        ty_name: &'static str,
        count: usize,
    ) -> &mut Self {
        let element_size = T::SHADER_SIZE.get();
        let element_alignment = T::min_size().get();
        let array_alignment = element_alignment;
        
        // Align array start
        let padding = padding_for_alignment(self.current_offset, array_alignment);
        self.current_offset += padding;
        
        // Calculate array size with proper stride
        let stride = align_size(element_size, element_alignment);
        let total_size = stride * count as u64;
        
        self.fields.push(FieldOffset {
            name,
            offset: self.current_offset,
            size: total_size,
            ty: format!("array<{}, {}>", ty_name, count),
        });
        
        self.current_offset += total_size;
        self.total_size = self.current_offset;
        
        self
    }
    
    /// Finalize with struct alignment
    pub fn build(mut self, struct_alignment: u64) -> LayoutInfo {
        // Pad to struct alignment
        let final_padding = padding_for_alignment(self.total_size, struct_alignment);
        self.total_size += final_padding;
        
        LayoutInfo {
            fields: self.fields,
            total_size: self.total_size,
            alignment: struct_alignment,
        }
    }
}

/// Complete layout information
#[derive(Debug)]
pub struct LayoutInfo {
    pub fields: Vec<FieldOffset>,
    pub total_size: u64,
    pub alignment: u64,
}

impl LayoutInfo {
    /// Generate a debug string showing the layout
    pub fn debug_string(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("Total size: {} bytes (aligned to {})\n", self.total_size, self.alignment));
        s.push_str("Fields:\n");
        
        for field in &self.fields {
            s.push_str(&format!(
                "  {:20} @ {:4} ({:4} bytes) : {}\n",
                field.name, field.offset, field.size, field.ty
            ));
        }
        
        s
    }
    
    /// Validate that a Rust struct matches this layout
    pub fn validate_rust_layout<T>(&self) -> Result<(), String> {
        let rust_size = mem::size_of::<T>() as u64;
        
        if rust_size != self.total_size {
            return Err(format!(
                "Size mismatch: Rust {} bytes, GPU {} bytes",
                rust_size, self.total_size
            ));
        }
        
        Ok(())
    }
}

/// Macro to automatically implement AutoLayout
#[macro_export]
macro_rules! impl_auto_layout {
    (
        $type:ty,
        fields = [
            $( $field:ident : $field_ty:ty = $field_name:literal ),* $(,)?
        ]
    ) => {
        impl $crate::gpu::automation::auto_layout::AutoLayout for $type {
            fn field_offsets() -> Vec<$crate::gpu::automation::auto_layout::FieldOffset> {
                let mut builder = $crate::gpu::automation::auto_layout::LayoutBuilder::new();
                
                $(
                    builder.add_field::<$field_ty>(
                        $field_name,
                        stringify!($field_ty)
                    );
                )*
                
                let layout = builder.build(16); // Standard WGSL alignment
                layout.fields
            }
        }
    };
}

/// Derive-like macro for automatic layout (simulated with macro_rules)
#[macro_export]
macro_rules! gpu_layout {
    (
        $(#[$meta:meta])*
        struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field:ident : $ty:ty
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[repr(C)]
        #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
        pub struct $name {
            $(
                $(#[$field_meta])*
                pub $field: $ty,
            )*
        }
        
        impl $crate::gpu::automation::auto_layout::AutoLayout for $name {
            fn field_offsets() -> Vec<$crate::gpu::automation::auto_layout::FieldOffset> {
                let mut builder = $crate::gpu::automation::auto_layout::LayoutBuilder::new();
                
                $(
                    builder.add_field::<$ty>(
                        stringify!($field),
                        stringify!($ty)
                    );
                )*
                
                let layout = builder.build(16); // Standard WGSL alignment
                layout.fields
            }
        }
    };
}

/// Generate layout constants for a type
pub fn generate_layout_constants<T: AutoLayout>(prefix: &str) -> String {
    let mut code = String::new();
    
    code.push_str(&format!("// Auto-generated layout constants for {}\n", prefix));
    code.push_str(&format!("pub const {}_SIZE: u64 = {};\n", prefix.to_uppercase(), T::gpu_size()));
    code.push_str(&format!("pub const {}_ALIGNMENT: u64 = {};\n", prefix.to_uppercase(), T::gpu_alignment()));
    code.push_str(&format!("pub const {}_STRIDE: u64 = {};\n", prefix.to_uppercase(), T::array_stride()));
    
    // Add field offsets
    code.push_str(&format!("\npub mod {}_offsets {{\n", prefix.to_lowercase()));
    for field in T::field_offsets() {
        code.push_str(&format!("    pub const {}: u64 = {};\n", field.name.to_uppercase(), field.offset));
    }
    code.push_str("}\n");
    
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Test struct
    gpu_layout! {
        struct TestVertex {
            position: [f32; 3],
            normal: [f32; 3],
            uv: [f32; 2],
            color: [f32; 4],
        }
    }
    
    #[test]
    fn test_auto_layout() {
        let offsets = TestVertex::field_offsets();
        
        // Check that fields have correct offsets
        assert_eq!(offsets[0].name, "position");
        assert_eq!(offsets[0].offset, 0);
        
        // Normal should be aligned to 16 bytes
        assert_eq!(offsets[1].name, "normal");
        assert_eq!(offsets[1].offset, 16);
        
        // Check total size
        let size = TestVertex::gpu_size();
        assert!(size > 0);
    }
    
    #[test]
    fn test_layout_builder() {
        let mut builder = LayoutBuilder::new();
        builder
            .add_field::<f32>("scale", "f32")
            .add_field::<[f32; 3]>("position", "vec3<f32>")
            .add_array::<u32>("indices", "u32", 16);
            
        let layout = builder.build(16);
        
        // Scale at offset 0
        assert_eq!(layout.fields[0].offset, 0);
        
        // Position aligned to 16 bytes
        assert_eq!(layout.fields[1].offset, 16);
        
        // Indices array after position
        assert!(layout.fields[2].offset >= 28);
    }
    
    #[test]
    fn test_alignment_calculations() {
        assert_eq!(align_size(5, 4), 8);
        assert_eq!(align_size(16, 16), 16);
        assert_eq!(align_size(17, 16), 32);
        
        assert_eq!(padding_for_alignment(0, 16), 0);
        assert_eq!(padding_for_alignment(1, 16), 15);
        assert_eq!(padding_for_alignment(16, 16), 0);
    }
}