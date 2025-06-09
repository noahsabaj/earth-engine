use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Performance metrics for tracking overall engine performance
#[derive(Clone)]
pub struct PerformanceMetrics {
    data: Arc<MetricsData>,
    start_time: Instant,
}

struct MetricsData {
    /// Frame count
    frames: AtomicU64,
    /// Total frame time in microseconds
    total_frame_time_us: AtomicU64,
    /// Chunk generation count
    chunks_generated: AtomicU64,
    /// Mesh builds count
    meshes_built: AtomicU64,
    /// Light updates count
    light_updates: AtomicU64,
    /// Cache line efficiency samples
    cache_efficiency_sum: AtomicU64,
    cache_efficiency_count: AtomicU64,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            data: Arc::new(MetricsData {
                frames: AtomicU64::new(0),
                total_frame_time_us: AtomicU64::new(0),
                chunks_generated: AtomicU64::new(0),
                meshes_built: AtomicU64::new(0),
                light_updates: AtomicU64::new(0),
                cache_efficiency_sum: AtomicU64::new(0),
                cache_efficiency_count: AtomicU64::new(0),
            }),
            start_time: Instant::now(),
        }
    }

    /// Record a frame
    pub fn record_frame(&self, frame_time: Duration) {
        self.data.frames.fetch_add(1, Ordering::Relaxed);
        self.data.total_frame_time_us.fetch_add(frame_time.as_micros() as u64, Ordering::Relaxed);
    }

    /// Record chunk generation
    pub fn record_chunk_generation(&self, count: u64) {
        self.data.chunks_generated.fetch_add(count, Ordering::Relaxed);
    }

    /// Record mesh build
    pub fn record_mesh_build(&self, count: u64) {
        self.data.meshes_built.fetch_add(count, Ordering::Relaxed);
    }

    /// Record light update
    pub fn record_light_update(&self, count: u64) {
        self.data.light_updates.fetch_add(count, Ordering::Relaxed);
    }

    /// Record cache efficiency sample (0-100)
    pub fn record_cache_efficiency(&self, efficiency_percent: u64) {
        self.data.cache_efficiency_sum.fetch_add(efficiency_percent, Ordering::Relaxed);
        self.data.cache_efficiency_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get average FPS
    pub fn average_fps(&self) -> f64 {
        let frames = self.data.frames.load(Ordering::Relaxed) as f64;
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            frames / elapsed
        } else {
            0.0
        }
    }

    /// Get average frame time in milliseconds
    pub fn average_frame_time_ms(&self) -> f64 {
        let frames = self.data.frames.load(Ordering::Relaxed);
        let total_us = self.data.total_frame_time_us.load(Ordering::Relaxed);
        if frames > 0 {
            (total_us as f64 / frames as f64) / 1000.0
        } else {
            0.0
        }
    }

    /// Get chunks per second
    pub fn chunks_per_second(&self) -> f64 {
        let chunks = self.data.chunks_generated.load(Ordering::Relaxed) as f64;
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            chunks / elapsed
        } else {
            0.0
        }
    }

    /// Get average cache efficiency
    pub fn average_cache_efficiency(&self) -> f64 {
        let sum = self.data.cache_efficiency_sum.load(Ordering::Relaxed);
        let count = self.data.cache_efficiency_count.load(Ordering::Relaxed);
        if count > 0 {
            sum as f64 / count as f64
        } else {
            0.0
        }
    }

    /// Print performance report
    pub fn report(&self) {
        let elapsed = self.start_time.elapsed();
        let frames = self.data.frames.load(Ordering::Relaxed);
        let chunks = self.data.chunks_generated.load(Ordering::Relaxed);
        let meshes = self.data.meshes_built.load(Ordering::Relaxed);
        let lights = self.data.light_updates.load(Ordering::Relaxed);

        println!("\n=== Performance Metrics Report ===");
        println!("Runtime: {:.2}s", elapsed.as_secs_f64());
        println!("\nFrame Performance:");
        println!("  Total frames: {}", frames);
        println!("  Average FPS: {:.2}", self.average_fps());
        println!("  Average frame time: {:.2}ms", self.average_frame_time_ms());
        
        println!("\nSystem Performance:");
        println!("  Chunks generated: {} ({:.2}/s)", chunks, self.chunks_per_second());
        println!("  Meshes built: {}", meshes);
        println!("  Light updates: {}", lights);
        
        println!("\nCache Performance:");
        println!("  Average efficiency: {:.2}%", self.average_cache_efficiency());
        
        println!("==================================\n");
    }

    /// Create a CSV header for logging
    pub fn csv_header() -> &'static str {
        "timestamp_ms,fps,frame_time_ms,chunks_per_sec,cache_efficiency"
    }

    /// Create a CSV row for current metrics
    pub fn csv_row(&self) -> String {
        format!(
            "{},{:.2},{:.2},{:.2},{:.2}",
            self.start_time.elapsed().as_millis(),
            self.average_fps(),
            self.average_frame_time_ms(),
            self.chunks_per_second(),
            self.average_cache_efficiency()
        )
    }
}