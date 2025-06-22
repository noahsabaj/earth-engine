//! Automatic WGSL generation using encase metadata
//!
//! This module provides automatic, type-safe WGSL generation from Rust types,
//! eliminating all manual string generation and ensuring perfect type alignment.

use crate::gpu::types::core::GpuData;
use encase::{ShaderSize, ShaderType};
use std::fmt::Write as FmtWrite;

/// Metadata for WGSL type generation
#[derive(Debug, Clone)]
pub struct WgslFieldMetadata {
    pub name: &'static str,
    pub wgsl_type: &'static str,
    pub offset: u32,
    pub size: u32,
    pub array_count: Option<usize>,
}

/// Trait for automatic WGSL generation from Rust types
pub trait AutoWgsl: GpuData {
    /// Get WGSL type name
    fn wgsl_name() -> &'static str;

    /// Get field metadata for WGSL generation
    fn wgsl_fields() -> Vec<WgslFieldMetadata>;

    /// Generate complete WGSL struct definition
    fn generate_wgsl() -> String {
        let mut wgsl = String::new();

        // Struct header with size/alignment info
        let size = Self::SHADER_SIZE.get();
        writeln!(&mut wgsl, "// Size: {} bytes, Alignment: 16 bytes", size)
            .expect("[AutoWgsl] writeln! to String should never fail");
        writeln!(&mut wgsl, "struct {} {{", Self::wgsl_name())
            .expect("[AutoWgsl] writeln! to String should never fail");

        // Generate fields
        let fields = Self::wgsl_fields();
        let mut current_offset = 0u32;

        for (i, field) in fields.iter().enumerate() {
            // Add padding if needed
            if field.offset > current_offset {
                let padding_size = field.offset - current_offset;
                let padding_type = padding_to_wgsl_type(padding_size);
                writeln!(&mut wgsl, "    _pad{}: {},", i, padding_type)
                    .expect("[AutoWgsl] writeln! to String should never fail");
            }

            // Add field
            if let Some(count) = field.array_count {
                writeln!(
                    &mut wgsl,
                    "    {}: array<{}, {}>,",
                    field.name, field.wgsl_type, count
                )
                .expect("[AutoWgsl] writeln! to String should never fail");
            } else {
                writeln!(&mut wgsl, "    {}: {},", field.name, field.wgsl_type)
                    .expect("[AutoWgsl] writeln! to String should never fail");
            }

            current_offset = field.offset + field.size;
        }

        // Final padding to match struct size
        let final_size = size as u32;
        if current_offset < final_size {
            let padding_size = final_size - current_offset;
            let padding_type = padding_to_wgsl_type(padding_size);
            writeln!(&mut wgsl, "    _pad_final: {},", padding_type)
                .expect("[AutoWgsl] writeln! to String should never fail");
        }

        wgsl.push_str("}");
        wgsl
    }
}

/// Convert padding size to appropriate WGSL type
fn padding_to_wgsl_type(size: u32) -> String {
    match size {
        4 => "u32".to_string(),
        8 => "vec2<u32>".to_string(),
        12 => "vec3<u32>".to_string(),
        16 => "vec4<u32>".to_string(),
        n if n % 16 == 0 => format!("array<vec4<u32>, {}>", n / 16),
        n if n % 4 == 0 => format!("array<u32, {}>", n / 4),
        _ => panic!("Invalid padding size: {} (must be multiple of 4)", size),
    }
}

/// Macro to implement AutoWgsl for types
#[macro_export]
macro_rules! auto_wgsl {
    (
        $type:ty,
        name = $wgsl_name:literal,
        fields = [
            $( $field_name:ident : $wgsl_type:literal $([ $array_count:expr ])? ),* $(,)?
        ]
    ) => {
        impl $crate::gpu::automation::auto_wgsl::AutoWgsl for $type {
            fn wgsl_name() -> &'static str {
                $wgsl_name
            }

            fn wgsl_fields() -> Vec<$crate::gpu::automation::auto_wgsl::WgslFieldMetadata> {
                vec![
                    $(
                        $crate::gpu::automation::auto_wgsl::WgslFieldMetadata {
                            name: stringify!($field_name),
                            wgsl_type: $wgsl_type,
                            offset: unsafe {
                                let base = std::ptr::null::<$type>();
                                let field = std::ptr::addr_of!((*base).$field_name);
                                field as usize as u32
                            },
                            size: {
                                // Calculate field size using pointer offsets to avoid dereferencing
                                // This creates a type with our field followed by a byte, then measures the gap
                                #[repr(C)]
                                struct FieldSizer<T> {
                                    _before: [u8; 0],
                                    field: T,
                                    _after: u8,
                                }
                                
                                unsafe {
                                    // Get offset of the byte after our field
                                    let base = std::ptr::null::<$type>();
                                    
                                    // Create an array of two identical structs to measure field size
                                    let arr = std::ptr::null::<[$type; 2]>();
                                    let field0 = std::ptr::addr_of!((*arr)[0].$field_name);
                                    let field1 = std::ptr::addr_of!((*arr)[1].$field_name);
                                    
                                    // Size is the distance between the same field in adjacent array elements
                                    let array_stride = field1 as usize - field0 as usize;
                                    
                                    // Get the actual field size by examining the specific field
                                    // For the last field, array_stride might include padding
                                    let next_field_offset = {
                                        // This is a bit tricky - we use the struct size vs field offset
                                        let struct_size = std::mem::size_of::<$type>();
                                        let field_offset = std::ptr::addr_of!((*base).$field_name) as usize;
                                        
                                        // Conservative estimate: assume field extends to the array stride point
                                        // unless it's the last field
                                        array_stride.min(struct_size - field_offset)
                                    };
                                    
                                    next_field_offset as u32
                                }
                            } as u32,
                            array_count: $crate::auto_wgsl!(@array_count $($array_count)?),
                        }
                    ),*
                ]
            }
        }
    };

    (@array_count) => { None };
    (@array_count $count:expr) => { Some($count) };
}

/// Generate WGSL bindings for a type
pub fn generate_bindings<T: AutoWgsl>(group: u32, binding: u32, access: &str) -> String {
    format!(
        "@group({}) @binding({}) var<storage, {}> {}: {};",
        group,
        binding,
        access,
        T::wgsl_name().to_lowercase(),
        T::wgsl_name()
    )
}

/// Generate WGSL function that operates on the type
pub fn generate_accessor_function<T: AutoWgsl>(field_name: &str) -> Option<String> {
    let fields = T::wgsl_fields();
    let field = fields.iter().find(|f| f.name == field_name)?;

    Some(format!(
        "fn get_{}_{} (data: ptr<storage, {}, read>) -> {} {{
    return (*data).{};
}}",
        T::wgsl_name().to_lowercase(),
        field_name,
        T::wgsl_name(),
        field.wgsl_type,
        field_name
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::{Pod, Zeroable};
    use encase::ShaderType;

    #[repr(C)]
    #[derive(ShaderType, Pod, Zeroable, Copy, Clone)]
    struct TestStruct {
        id: u32,
        _pad1: [u32; 3],
        position: [f32; 3],
        scale: f32,
    }

    auto_wgsl!(
        TestStruct,
        name = "TestStruct",
        fields = [
            id: "u32",
            position: "vec3<f32>",
            scale: "f32",
        ]
    );

    #[test]
    fn test_auto_wgsl_generation() {
        let wgsl = TestStruct::generate_wgsl();
        assert!(wgsl.contains("struct TestStruct"));
        assert!(wgsl.contains("id: u32"));
        assert!(wgsl.contains("position: vec3<f32>"));
        assert!(wgsl.contains("scale: f32"));
    }

    #[test]
    fn test_binding_generation() {
        let binding = generate_bindings::<TestStruct>(0, 1, "read");
        assert_eq!(
            binding,
            "@group(0) @binding(1) var<storage, read> teststruct: TestStruct;"
        );
    }
}
