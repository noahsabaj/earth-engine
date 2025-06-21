//! Embedded shader includes for cross-platform compatibility
//! 
//! This module provides shader includes that are embedded at compile time
//! to avoid runtime path resolution issues on different platforms.
//!
//! NOTE: Generated types and constants are now handled by the unified GPU system at runtime.

/// Perlin noise functions for terrain generation
pub const PERLIN_NOISE_WGSL: &str = include_str!("../shaders/rendering/perlin_noise.wgsl");

/// Unified Morton encoding functions for GPU shaders
pub const MORTON_WGSL: &str = include_str!("wgsl_includes/morton.wgsl");

/// Get shader include content by name
pub fn get_shader_include(name: &str) -> Option<&'static str> {
    match name {
        // Generated files are now handled by the unified GPU system at runtime
        "types_soa.wgsl" | "generated/types_soa.wgsl" | "../generated/types_soa.wgsl" => {
            None // Unified GPU system provides these
        }
        "constants.wgsl" | "generated/constants.wgsl" | "../generated/constants.wgsl" => {
            None // Unified GPU system provides these
        }
        "perlin_noise.wgsl" | "../../../renderer/shaders/perlin_noise.wgsl" => {
            Some(PERLIN_NOISE_WGSL)
        }
        "morton.wgsl" | "wgsl_includes/morton.wgsl" => {
            Some(MORTON_WGSL)
        }
        _ => None,
    }
}