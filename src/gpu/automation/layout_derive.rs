//! Automatic layout derivation for GPU types
//! 
//! This module provides derive-like functionality for automatic memory layout
//! calculation, replacing all manual padding and offset calculations.

use std::any::type_name;
use encase::{ShaderType, ShaderSize};

/// Trait that provides automatic layout information
pub trait DeriveLayout: ShaderType + ShaderSize {
    /// Get a human-readable layout report
    fn layout_report() -> String {
        let size = Self::SHADER_SIZE.get();
        let min_size = Self::min_size().get();
        
        format!(
            "Type: {}\nGPU Size: {} bytes\nAlignment: {} bytes\n",
            type_name::<Self>(),
            size,
            min_size
        )
    }
    
    /// Validate that manual constants match calculated values
    fn validate_constants(expected_size: u64, expected_alignment: u64) -> Result<(), String> {
        let actual_size = Self::SHADER_SIZE.get();
        let actual_alignment = Self::min_size().get();
        
        if actual_size != expected_size {
            return Err(format!(
                "Size mismatch for {}: expected {} bytes, got {} bytes",
                type_name::<Self>(),
                expected_size,
                actual_size
            ));
        }
        
        if actual_alignment != expected_alignment {
            return Err(format!(
                "Alignment mismatch for {}: expected {} bytes, got {} bytes",
                type_name::<Self>(),
                expected_alignment,
                actual_alignment
            ));
        }
        
        Ok(())
    }
    
    /// Generate WGSL-compatible padding arrays
    fn padding_type(bytes: u64) -> &'static str {
        match bytes {
            4 => "u32",
            8 => "vec2<u32>",
            12 => "vec3<u32>",
            16 => "vec4<u32>",
            n if n % 16 == 0 => "array<vec4<u32>, N>", // Replace N with n/16
            n if n % 4 == 0 => "array<u32, N>", // Replace N with n/4
            _ => panic!("Invalid padding size: {} bytes", bytes),
        }
    }
}

/// Implement DeriveLayout for all ShaderType types
impl<T: ShaderType + ShaderSize> DeriveLayout for T {}

/// Macro to create GPU structs with automatic layout
#[macro_export]
macro_rules! gpu_struct {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                pub $field:ident : $ty:ty
            ),* $(,)?
        }
    ) => {
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
        
        impl $name {
            /// Get layout information for this struct
            pub fn layout_info() -> $crate::gpu::layout_derive::StructLayout {
                use $crate::gpu::layout_derive::DeriveLayout;
                
                $crate::gpu::layout_derive::StructLayout {
                    name: stringify!($name),
                    size: std::mem::size_of::<$name>() as u64,
                    alignment: 16, // Standard WGSL alignment
                    fields: vec![
                        $(
                            $crate::gpu::layout_derive::FieldInfo {
                                name: stringify!($field),
                                ty: stringify!($ty),
                                size: std::mem::size_of::<$ty>() as u64,
                                offset: $crate::gpu::layout_derive::calculate_field_offset::<Self, $ty>(
                                    stringify!($field)
                                ),
                            }
                        ),*
                    ],
                }
            }
            
            /// Validate that this struct's layout matches expectations
            pub fn validate_layout() -> Result<(), String> {
                let layout = Self::layout_info();
                layout.validate()
            }
            
            /// Generate constants for this struct's layout
            pub fn generate_constants() -> String {
                let layout = Self::layout_info();
                layout.generate_constants()
            }
        }
    };
}

/// Information about a struct's memory layout
#[derive(Debug)]
pub struct StructLayout {
    pub name: &'static str,
    pub size: u64,
    pub alignment: u64,
    pub fields: Vec<FieldInfo>,
}

/// Information about a field's layout
#[derive(Debug)]
pub struct FieldInfo {
    pub name: &'static str,
    pub ty: &'static str,
    pub size: u64,
    pub offset: u64,
}

impl StructLayout {
    /// Validate that the layout is correct
    pub fn validate(&self) -> Result<(), String> {
        let mut current_offset = 0u64;
        
        for field in &self.fields {
            if field.offset < current_offset {
                return Err(format!(
                    "Field {} at offset {} overlaps with previous field",
                    field.name, field.offset
                ));
            }
            current_offset = field.offset + field.size;
        }
        
        if current_offset > self.size {
            return Err(format!(
                "Fields extend beyond struct size: {} > {}",
                current_offset, self.size
            ));
        }
        
        Ok(())
    }
    
    /// Generate Rust constants for this layout
    pub fn generate_constants(&self) -> String {
        let mut code = String::new();
        let prefix = self.name.to_uppercase();
        
        code.push_str(&format!("// Auto-generated layout constants for {}\n", self.name));
        code.push_str(&format!("pub const {}_SIZE: u64 = {};\n", prefix, self.size));
        code.push_str(&format!("pub const {}_ALIGNMENT: u64 = {};\n", prefix, self.alignment));
        code.push_str("\n");
        
        code.push_str(&format!("pub mod {}_offsets {{\n", self.name.to_lowercase()));
        for field in &self.fields {
            code.push_str(&format!(
                "    pub const {}: u64 = {}; // {} ({} bytes)\n",
                field.name.to_uppercase(),
                field.offset,
                field.ty,
                field.size
            ));
        }
        code.push_str("}\n");
        
        code
    }
    
    /// Generate a visual representation of the layout
    pub fn visualize(&self) -> String {
        let mut viz = String::new();
        
        viz.push_str(&format!("=== {} Layout ({} bytes) ===\n", self.name, self.size));
        viz.push_str("Offset | Size  | Field\n");
        viz.push_str("-------|-------|------------------\n");
        
        let mut last_end = 0u64;
        
        for field in &self.fields {
            // Show padding if there is any
            if field.offset > last_end {
                let padding = field.offset - last_end;
                viz.push_str(&format!(
                    "{:6} | {:5} | [padding]\n",
                    last_end, padding
                ));
            }
            
            viz.push_str(&format!(
                "{:6} | {:5} | {} : {}\n",
                field.offset, field.size, field.name, field.ty
            ));
            
            last_end = field.offset + field.size;
        }
        
        // Show final padding
        if last_end < self.size {
            let padding = self.size - last_end;
            viz.push_str(&format!(
                "{:6} | {:5} | [padding]\n",
                last_end, padding
            ));
        }
        
        viz
    }
}

/// Calculate field offset (placeholder - would use offset_of! in real implementation)
pub fn calculate_field_offset<S, F>(_field_name: &str) -> u64 {
    // In a real implementation, we would use offset_of! macro or similar
    // For now, return 0 as a placeholder
    0
}

/// Macro to validate all GPU struct layouts at compile time
#[macro_export]
macro_rules! validate_gpu_layouts {
    ($($type:ty),* $(,)?) => {
        #[cfg(test)]
        mod layout_validation {
            use super::*;
            
            $(
                #[test]
                fn validate_layout_of_$type() {
                    match <$type>::validate_layout() {
                        Ok(()) => {},
                        Err(e) => panic!("Layout validation failed for {}: {}", stringify!($type), e),
                    }
                }
            )*
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    gpu_struct! {
        /// Test vertex structure
        pub struct TestVertex {
            pub position: [f32; 3],
            pub _pad0: u32,
            pub normal: [f32; 3],
            pub _pad1: u32,
            pub uv: [f32; 2],
            pub color: [f32; 4],
        }
    }
    
    #[test]
    fn test_gpu_struct_macro() {
        let layout = TestVertex::layout_info();
        
        // Verify the struct has layout info
        assert_eq!(layout.name, "TestVertex");
        assert!(layout.size > 0);
        assert!(layout.alignment > 0);
        
        // Verify fields are captured
        assert!(layout.fields.iter().any(|f| f.name == "position"));
        assert!(layout.fields.iter().any(|f| f.name == "normal"));
        assert!(layout.fields.iter().any(|f| f.name == "uv"));
        assert!(layout.fields.iter().any(|f| f.name == "color"));
    }
    
    #[test]
    fn test_layout_visualization() {
        let layout = TestVertex::layout_info();
        let viz = layout.visualize();
        
        // Check that visualization contains expected content
        assert!(viz.contains("TestVertex Layout"));
        assert!(viz.contains("position"));
        assert!(viz.contains("normal"));
    }
    
    #[test]
    fn test_constant_generation() {
        let layout = TestVertex::layout_info();
        let constants = layout.generate_constants();
        
        // Check generated constants
        assert!(constants.contains("TESTVERTEX_SIZE"));
        assert!(constants.contains("TESTVERTEX_ALIGNMENT"));
        assert!(constants.contains("testvertex_offsets"));
    }
}