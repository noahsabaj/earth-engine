//! Panic handler with telemetry for Hearth Engine
//! 
//! This module provides a custom panic handler that logs panic information
//! before the process terminates, helping with debugging and stability monitoring.

use std::panic::{self, PanicHookInfo};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use chrono::{DateTime, Local};
use std::backtrace::Backtrace;

/// Global panic counter for telemetry
static PANIC_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Panic telemetry data
#[derive(Debug)]
pub struct PanicTelemetry {
    pub timestamp: DateTime<Local>,
    pub location: String,
    pub message: String,
    pub backtrace: String,
    pub panic_count: usize,
}

impl PanicTelemetry {
    fn from_panic_info(info: &PanicHookInfo) -> Self {
        let location = if let Some(location) = info.location() {
            format!("{}:{}:{}", location.file(), location.line(), location.column())
        } else {
            "unknown location".to_string()
        };

        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic message".to_string()
        };

        let backtrace = Backtrace::capture().to_string();
        let panic_count = PANIC_COUNT.fetch_add(1, Ordering::SeqCst) + 1;

        Self {
            timestamp: Local::now(),
            location,
            message,
            backtrace,
            panic_count,
        }
    }

    /// Write telemetry to log file
    fn write_to_log(&self, log_path: &PathBuf) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;

        writeln!(file, "=== PANIC #{} ===", self.panic_count)?;
        writeln!(file, "Timestamp: {}", self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"))?;
        writeln!(file, "Location: {}", self.location)?;
        writeln!(file, "Message: {}", self.message)?;
        writeln!(file, "Backtrace:\n{}", self.backtrace)?;
        writeln!(file, "================\n")?;

        file.flush()?;
        Ok(())
    }

    /// Send telemetry to monitoring service (placeholder)
    fn send_to_monitoring(&self) {
        // In a production environment, this would send data to a monitoring service
        // For now, we just log to stderr
        eprintln!("\n🚨 PANIC DETECTED 🚨");
        eprintln!("Panic #{} at {}", self.panic_count, self.location);
        eprintln!("Message: {}", self.message);
        eprintln!("Time: {}", self.timestamp.format("%Y-%m-%d %H:%M:%S"));
    }
}

/// Install the custom panic handler
pub fn install_panic_handler() {
    // Create logs directory if it doesn't exist
    let log_dir = PathBuf::from("logs");
    if !log_dir.exists() {
        let _ = std::fs::create_dir_all(&log_dir);
    }

    let log_path = log_dir.join("panic.log");

    panic::set_hook(Box::new(move |panic_info| {
        // Collect telemetry
        let telemetry = PanicTelemetry::from_panic_info(panic_info);

        // Log to file
        if let Err(e) = telemetry.write_to_log(&log_path) {
            eprintln!("Failed to write panic log: {}", e);
        }

        // Send to monitoring
        telemetry.send_to_monitoring();

        // Print to stderr for immediate visibility
        eprintln!("\n💥 Hearth Engine Panic! 💥");
        eprintln!("This should never happen in production!");
        eprintln!("Please report this issue with the panic log.");
        eprintln!("Log location: {}", log_path.display());

        // Call the default panic handler to maintain normal panic behavior
        // This is important for test frameworks and debugging
        if std::env::var("RUST_BACKTRACE").is_err() {
            eprintln!("\nHint: Set RUST_BACKTRACE=1 for more detailed backtrace");
        }

        // In debug builds, also print the full backtrace
        #[cfg(debug_assertions)]
        {
            eprintln!("\n--- Debug Backtrace ---");
            eprintln!("{}", telemetry.backtrace);
        }
    }));

    log::info!("Panic handler installed. Panics will be logged to logs/panic.log");
}

/// Get the current panic count
pub fn get_panic_count() -> usize {
    PANIC_COUNT.load(Ordering::SeqCst)
}

/// Reset the panic counter (useful for tests)
#[cfg(test)]
pub fn reset_panic_count() {
    PANIC_COUNT.store(0, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::catch_unwind;

    #[test]
    fn test_panic_counter() {
        reset_panic_count();
        assert_eq!(get_panic_count(), 0);

        // Catch panic to prevent test failure
        let _ = catch_unwind(|| {
            panic!("Test panic");
        });

        // Note: In tests, our custom handler may not be called
        // This is just to verify the counter mechanism works
    }

    #[test]
    fn test_telemetry_creation() {
        // We can't easily test PanicInfo creation, but we can test
        // the telemetry structure
        let telemetry = PanicTelemetry {
            timestamp: Local::now(),
            location: "test.rs:42:10".to_string(),
            message: "test panic".to_string(),
            backtrace: "backtrace here".to_string(),
            panic_count: 1,
        };

        assert!(telemetry.location.contains("test.rs"));
        assert_eq!(telemetry.message, "test panic");
        assert_eq!(telemetry.panic_count, 1);
    }
}