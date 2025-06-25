//! Pure data structures for GPU progress tracking - NO METHODS!
//! All operations are in renderer_operations.rs

use crate::renderer::renderer_data::{
    GpuInitProgressData, ProgressStepData, StepStatus,
    AsyncProgressReporterData, LogProgressCallbackData
};

// Type aliases for clarity
pub type GpuInitProgress = GpuInitProgressData;
pub type ProgressStep = ProgressStepData;
pub type AsyncProgressReporter = AsyncProgressReporterData;
pub type LogProgressCallback = LogProgressCallbackData;

// Re-export status enum
pub use crate::renderer::renderer_data::StepStatus as ProgressStepStatus;

/// Progress callback trait - moved here as it's an interface
pub trait ProgressCallback: Send + Sync {
    fn on_progress(&self, current: usize, total: usize, message: &str);
    fn on_complete(&self, message: &str);
    fn on_error(&self, error: &str);
}

// Timeout wrapper functions are in renderer_operations.rs