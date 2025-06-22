use crate::error::EngineError;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// GPU initialization progress tracking module
///
/// ## Error Handling Pattern
/// This module demonstrates proper lock-based error handling:
/// - Mutex::lock() operations use the ? operator instead of unwrap()
/// - All methods return Result<T, EngineError> for error propagation
/// - Lock poisoning is properly handled through the EngineError::LockPoisoned variant
/// - No panic-prone operations remain in production code paths

#[cfg(target_arch = "wasm32")]
use instant as time_instant;

/// GPU initialization progress tracker
pub struct GpuInitProgress {
    steps: Arc<Mutex<Vec<ProgressStep>>>,
    current_step: Arc<Mutex<usize>>,
    start_time: Instant,
}

#[derive(Clone)]
pub struct ProgressStep {
    pub name: String,
    pub status: StepStatus,
    pub duration: Option<Duration>,
    pub details: Option<String>,
}

#[derive(Clone, Debug)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
    Warning(String),
}

impl GpuInitProgress {
    pub fn new() -> Self {
        let steps = vec![
            ProgressStep::new("Create WGPU Instance"),
            ProgressStep::new("Run GPU Diagnostics"),
            ProgressStep::new("Create Surface"),
            ProgressStep::new("Request Adapter"),
            ProgressStep::new("Validate Capabilities"),
            ProgressStep::new("Create Device"),
            ProgressStep::new("Test GPU Operations"),
            ProgressStep::new("Configure Surface"),
            ProgressStep::new("Create Depth Texture"),
            ProgressStep::new("Create Render Pipeline"),
            ProgressStep::new("Initialize World"),
            ProgressStep::new("Create Renderers"),
        ];

        Self {
            steps: Arc::new(Mutex::new(steps)),
            current_step: Arc::new(Mutex::new(0)),
            start_time: Instant::now(),
        }
    }

    /// Start a new step
    pub fn start_step(&self, name: &str) -> Result<(), EngineError> {
        let mut steps = self.steps.lock()?;
        let current = self.current_step.lock()?;

        if let Some(step) = steps.iter_mut().find(|s| s.name == name) {
            step.status = StepStatus::InProgress;
            step.details = None;
            log::info!(
                "[GPU Init Progress] Step {}/{}: {} - Starting...",
                *current + 1,
                steps.len(),
                name
            );
        }

        Ok(())
    }

    /// Complete current step
    pub fn complete_step(&self, name: &str, details: Option<String>) -> Result<(), EngineError> {
        let mut steps = self.steps.lock()?;
        let mut current = self.current_step.lock()?;

        if let Some(step) = steps.iter_mut().find(|s| s.name == name) {
            let duration = self.start_time.elapsed();
            step.status = StepStatus::Completed;
            step.duration = Some(duration);
            step.details = details.clone();

            *current += 1;

            let elapsed = duration.as_secs_f32();
            if let Some(details) = details {
                log::info!(
                    "[GPU Init Progress] Step {}/{}: {} - Completed in {:.2}s ({})",
                    *current,
                    steps.len(),
                    name,
                    elapsed,
                    details
                );
            } else {
                log::info!(
                    "[GPU Init Progress] Step {}/{}: {} - Completed in {:.2}s",
                    *current,
                    steps.len(),
                    name,
                    elapsed
                );
            }
        }

        Ok(())
    }

    /// Fail current step
    pub fn fail_step(&self, name: &str, error: String) -> Result<(), EngineError> {
        let mut steps = self.steps.lock()?;

        if let Some(step) = steps.iter_mut().find(|s| s.name == name) {
            step.status = StepStatus::Failed(error.clone());
            step.duration = Some(self.start_time.elapsed());

            log::error!("[GPU Init Progress] Step FAILED: {} - {}", name, error);
        }

        Ok(())
    }

    /// Add warning to current step
    pub fn warn_step(&self, name: &str, warning: String) -> Result<(), EngineError> {
        let mut steps = self.steps.lock()?;

        if let Some(step) = steps.iter_mut().find(|s| s.name == name) {
            step.status = StepStatus::Warning(warning.clone());
            log::warn!("[GPU Init Progress] Step WARNING: {} - {}", name, warning);
        }

        Ok(())
    }

    /// Get progress percentage
    pub fn get_progress(&self) -> Result<f32, EngineError> {
        let steps = self.steps.lock()?;
        let completed = steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Completed))
            .count();
        Ok((completed as f32 / steps.len() as f32) * 100.0)
    }

    /// Print summary
    pub fn print_summary(&self) -> Result<(), EngineError> {
        let steps = self.steps.lock()?;
        let total_time = self.start_time.elapsed();

        log::info!("=== GPU Initialization Summary ===");
        log::info!("Total time: {:.2}s", total_time.as_secs_f32());

        let completed = steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Completed))
            .count();
        let failed = steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Failed(_)))
            .count();
        let warnings = steps
            .iter()
            .filter(|s| matches!(s.status, StepStatus::Warning(_)))
            .count();

        let progress = (completed as f32 / steps.len() as f32) * 100.0;
        log::info!("Progress: {:.1}%", progress);
        log::info!(
            "Steps: {} completed, {} failed, {} warnings",
            completed,
            failed,
            warnings
        );

        if failed > 0 {
            log::error!("\nFailed steps:");
            for step in steps
                .iter()
                .filter(|s| matches!(s.status, StepStatus::Failed(_)))
            {
                if let StepStatus::Failed(error) = &step.status {
                    log::error!("  - {}: {}", step.name, error);
                }
            }
        }

        if warnings > 0 {
            log::warn!("\nWarnings:");
            for step in steps
                .iter()
                .filter(|s| matches!(s.status, StepStatus::Warning(_)))
            {
                if let StepStatus::Warning(warning) = &step.status {
                    log::warn!("  - {}: {}", step.name, warning);
                }
            }
        }

        Ok(())
    }

    /// Check if initialization should continue
    pub fn should_continue(&self) -> Result<bool, EngineError> {
        let steps = self.steps.lock()?;
        Ok(!steps
            .iter()
            .any(|s| matches!(s.status, StepStatus::Failed(_))))
    }
}

impl ProgressStep {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: StepStatus::Pending,
            duration: None,
            details: None,
        }
    }
}

/// Async progress reporter for long operations
pub struct AsyncProgressReporter {
    name: String,
    start_time: Instant,
    last_report: Instant,
    report_interval: Duration,
}

impl AsyncProgressReporter {
    pub fn new(name: &str) -> Self {
        let now = Instant::now();
        Self {
            name: name.to_string(),
            start_time: now,
            last_report: now,
            report_interval: Duration::from_secs(1),
        }
    }

    /// Report progress if enough time has passed
    pub fn report_progress(&mut self, current: usize, total: usize) {
        let now = Instant::now();
        if now.duration_since(self.last_report) >= self.report_interval {
            let elapsed = now.duration_since(self.start_time);
            let progress = (current as f32 / total as f32) * 100.0;
            log::info!(
                "[{}] Progress: {:.1}% ({}/{}) - Elapsed: {:.1}s",
                self.name,
                progress,
                current,
                total,
                elapsed.as_secs_f32()
            );
            self.last_report = now;
        }
    }

    /// Final report
    pub fn finish(self, total: usize) {
        let elapsed = Instant::now().duration_since(self.start_time);
        log::info!(
            "[{}] Completed {} items in {:.2}s",
            self.name,
            total,
            elapsed.as_secs_f32()
        );
    }
}

/// Timeout wrapper for async operations
#[cfg(feature = "native")]
pub async fn with_timeout<T, F>(
    operation_name: &str,
    timeout: Duration,
    future: F,
) -> Result<T, String>
where
    F: std::future::Future<Output = T> + Unpin,
{
    // Use futures::future::select and a timer future for timeout without requiring tokio runtime
    use futures::future::{select, Either};
    use futures_timer::Delay;

    let timeout_future = Delay::new(timeout);

    match select(future, timeout_future).await {
        Either::Left((result, _)) => Ok(result),
        Either::Right((_, _)) => {
            let error = format!("{} timed out after {:?}", operation_name, timeout);
            log::error!("[GPU Timeout] {}", error);
            Err(error)
        }
    }
}

/// Timeout wrapper for async operations (web/no-tokio version)
#[cfg(not(feature = "native"))]
pub async fn with_timeout<T, F>(
    operation_name: &str,
    timeout: Duration,
    future: F,
) -> Result<T, String>
where
    F: std::future::Future<Output = T>,
{
    // On web, we don't have tokio, so just run the future directly
    log::warn!("[GPU Timeout] Timeout not implemented for web platform");
    Ok(future.await)
}

/// Progress callback for operations that support it
pub trait ProgressCallback: Send + Sync {
    fn on_progress(&self, current: usize, total: usize, message: &str);
    fn on_complete(&self, message: &str);
    fn on_error(&self, error: &str);
}

/// Simple progress callback implementation
pub struct LogProgressCallback {
    name: String,
}

impl LogProgressCallback {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl ProgressCallback for LogProgressCallback {
    fn on_progress(&self, current: usize, total: usize, message: &str) {
        let progress = (current as f32 / total as f32) * 100.0;
        log::info!("[{}] {:.1}% - {}", self.name, progress, message);
    }

    fn on_complete(&self, message: &str) {
        log::info!("[{}] Complete - {}", self.name, message);
    }

    fn on_error(&self, error: &str) {
        log::error!("[{}] Error - {}", self.name, error);
    }
}
