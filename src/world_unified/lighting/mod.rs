//! Lighting system for the unified world architecture
//! 
//! This module provides lighting calculations optimized for GPU processing
//! with CPU fallbacks for compatibility.

mod skylight;

pub use skylight::SkylightCalculator;