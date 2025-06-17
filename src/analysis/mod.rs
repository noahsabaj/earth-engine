//! Analysis modules for Hearth Engine performance and feasibility studies

pub mod voxel_size_impact_analysis;
pub mod voxel_size_benchmark;
pub mod gpu_architecture_reality;
pub mod fps_crisis_analyzer;

pub use voxel_size_benchmark::VoxelSizeBenchmark;
pub use gpu_architecture_reality::{GpuArchitectureReality, GpuOperationAnalyzer};
pub use fps_crisis_analyzer::{FpsCrisisAnalyzer, analyze_fps_crisis};