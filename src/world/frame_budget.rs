use std::time::{Duration, Instant};

/// Frame time budget manager for chunk loading
pub struct FrameBudget {
    frame_start: Instant,
    target_frame_time: Duration,
    max_frame_time: Duration,
    time_spent: Duration,
}

impl FrameBudget {
    /// Create a new frame budget targeting 60 FPS
    pub fn new() -> Self {
        Self::with_target_fps(60)
    }
    
    /// Create a frame budget with a specific target FPS
    pub fn with_target_fps(fps: u32) -> Self {
        let target_frame_time = Duration::from_secs_f32(1.0 / fps as f32);
        let max_frame_time = target_frame_time.mul_f32(0.5); // Use at most 50% of frame time
        
        Self {
            frame_start: Instant::now(),
            target_frame_time,
            max_frame_time,
            time_spent: Duration::ZERO,
        }
    }
    
    /// Start a new frame
    pub fn start_frame(&mut self) {
        self.frame_start = Instant::now();
        self.time_spent = Duration::ZERO;
    }
    
    /// Check if there's still time budget available
    pub fn has_budget(&self) -> bool {
        let elapsed = self.frame_start.elapsed();
        elapsed < self.max_frame_time
    }
    
    /// Get remaining time in this frame's budget
    pub fn remaining_budget(&self) -> Duration {
        let elapsed = self.frame_start.elapsed();
        if elapsed >= self.max_frame_time {
            Duration::ZERO
        } else {
            self.max_frame_time - elapsed
        }
    }
    
    /// Record time spent on an operation
    pub fn record_time(&mut self, duration: Duration) {
        self.time_spent += duration;
    }
    
    /// Get the percentage of frame time used
    pub fn usage_percentage(&self) -> f32 {
        let elapsed = self.frame_start.elapsed();
        (elapsed.as_secs_f32() / self.target_frame_time.as_secs_f32()) * 100.0
    }
}

/// Chunk loading throttler with frame time budgeting
pub struct ChunkLoadThrottler {
    budget: FrameBudget,
    chunks_per_frame_limit: usize,
    adaptive_mode: bool,
    min_chunks_per_frame: usize,
    max_chunks_per_frame: usize,
    current_chunks_per_frame: usize,
}

impl ChunkLoadThrottler {
    pub fn new() -> Self {
        Self {
            budget: FrameBudget::new(),
            chunks_per_frame_limit: 5,
            adaptive_mode: true,
            min_chunks_per_frame: 1,
            max_chunks_per_frame: 10,
            current_chunks_per_frame: 5,
        }
    }
    
    /// Start a new frame
    pub fn start_frame(&mut self) {
        self.budget.start_frame();
        
        // Adaptive adjustment based on previous frame
        if self.adaptive_mode {
            let usage = self.budget.usage_percentage();
            if usage < 30.0 && self.current_chunks_per_frame < self.max_chunks_per_frame {
                self.current_chunks_per_frame += 1;
            } else if usage > 60.0 && self.current_chunks_per_frame > self.min_chunks_per_frame {
                self.current_chunks_per_frame -= 1;
            }
        }
    }
    
    /// Check if we can load another chunk this frame
    pub fn can_load_chunk(&self) -> bool {
        self.budget.has_budget()
    }
    
    /// Record the time taken to load a chunk
    pub fn record_chunk_load(&mut self, duration: Duration) {
        self.budget.record_time(duration);
    }
    
    /// Get the current chunks per frame limit
    pub fn get_chunks_per_frame(&self) -> usize {
        if self.adaptive_mode {
            self.current_chunks_per_frame
        } else {
            self.chunks_per_frame_limit
        }
    }
    
    /// Set adaptive mode
    pub fn set_adaptive_mode(&mut self, enabled: bool) {
        self.adaptive_mode = enabled;
    }
    
    /// Set fixed chunks per frame limit
    pub fn set_chunks_per_frame(&mut self, limit: usize) {
        self.chunks_per_frame_limit = limit.max(1);
        if !self.adaptive_mode {
            self.current_chunks_per_frame = self.chunks_per_frame_limit;
        }
    }
}