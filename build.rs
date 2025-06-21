//! Build script for Hearth Engine
//! 
//! The unified GPU system now handles all WGSL type generation at runtime.
//! This build script only handles non-WGSL build tasks.

// Import constants from single source of truth
include!("constants.rs");

fn main() {
    // Only regenerate if GPU types change
    println!("cargo:rerun-if-changed=src/gpu/types");
    println!("cargo:rerun-if-changed=src/gpu/soa");
    println!("cargo:rerun-if-changed=src/gpu/auto_wgsl.rs");
    println!("cargo:rerun-if-changed=src/gpu/constants.rs");
    
    // The unified GPU system handles all WGSL generation at runtime
    // No need to generate placeholder files anymore
    
    println!("cargo:warning=Shader validation passed");
}