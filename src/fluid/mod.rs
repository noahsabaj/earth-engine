/// GPU Fluid Dynamics System
/// 
/// Pure data-oriented fluid simulation running entirely on GPU.
/// Integrates with the WorldBuffer architecture from Sprint 21
/// and streaming system from Sprint 23.

pub mod fluid_data;
pub mod fluid_compute;
pub mod pressure_solver;
pub mod multi_phase;
pub mod terrain_interaction;
pub mod fluid_renderer;
pub mod performance;

pub use fluid_data::{FluidVoxel, FluidType, FluidBuffer, FluidConstants};
pub use fluid_compute::{FluidCompute, FluidPipeline};
pub use pressure_solver::{PressureSolver, FlowField};
pub use multi_phase::{PhaseSystem, FluidPhase};
pub use terrain_interaction::{TerrainInteraction, ErosionParams};
pub use fluid_renderer::{FluidRenderer, FluidRenderParams};
pub use performance::{FluidPerformanceMonitor, FluidPerformanceMetrics};

/// Maximum fluid velocity (units per second)
pub const MAX_FLUID_VELOCITY: f32 = 10.0;

/// Fluid simulation time step
pub const FLUID_TIME_STEP: f32 = 0.016; // 60 FPS

/// Number of pressure solver iterations
pub const PRESSURE_ITERATIONS: u32 = 20;

/// Fluid cell size (must match voxel size)
pub const FLUID_CELL_SIZE: f32 = 1.0;

#[cfg(test)]
mod tests;