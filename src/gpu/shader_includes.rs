//! Embedded shader includes for cross-platform compatibility
//! 
//! This module provides shader includes that are embedded at compile time
//! to avoid runtime path resolution issues on different platforms.

/// The auto-generated WGSL types from build.rs
pub const GENERATED_TYPES_WGSL: &str = include_str!("shaders/generated/types.wgsl");

/// Get shader include content by name
pub fn get_shader_include(name: &str) -> Option<&'static str> {
    match name {
        "types.wgsl" | "generated/types.wgsl" | "../../gpu/shaders/generated/types.wgsl" => {
            Some(GENERATED_TYPES_WGSL)
        }
        _ => None,
    }
}