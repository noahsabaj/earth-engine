//! Embedded shader includes for cross-platform compatibility
//! 
//! This module provides shader includes that are embedded at compile time
//! to avoid runtime path resolution issues on different platforms.

/// The auto-generated SOA WGSL types from build.rs
pub const GENERATED_TYPES_SOA_WGSL: &str = include_str!("shaders/generated/types_soa.wgsl");

/// The auto-generated constants from build.rs
pub const GENERATED_CONSTANTS_WGSL: &str = include_str!("shaders/generated/constants.wgsl");

/// Perlin noise functions for terrain generation
pub const PERLIN_NOISE_WGSL: &str = include_str!("../renderer/shaders/perlin_noise.wgsl");

/// Get shader include content by name
pub fn get_shader_include(name: &str) -> Option<&'static str> {
    match name {
        "types_soa.wgsl" | "generated/types_soa.wgsl" | "../generated/types_soa.wgsl" => {
            Some(GENERATED_TYPES_SOA_WGSL)
        }
        "constants.wgsl" | "generated/constants.wgsl" | "../generated/constants.wgsl" => {
            Some(GENERATED_CONSTANTS_WGSL)
        }
        "perlin_noise.wgsl" | "../../../renderer/shaders/perlin_noise.wgsl" => {
            Some(PERLIN_NOISE_WGSL)
        }
        _ => None,
    }
}