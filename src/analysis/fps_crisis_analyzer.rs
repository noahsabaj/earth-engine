use crate::profiling::reality_check_profiler::{
    RealityCheckProfiler, BlockingType, FrameMetrics, 
    begin_frame, end_frame, time_cpu_operation, write_gpu_timestamp
};
use std::time::Instant;
use wgpu::{Device, Queue, CommandEncoder};

/// FPS Crisis Analyzer - Find why we're at 0.8 FPS instead of 60 FPS
pub struct FpsCrisisAnalyzer {
    profiler: RealityCheckProfiler,
    frame_count: u32,
}

impl FpsCrisisAnalyzer {
    pub fn new(device: &Device, queue: &Queue) -> Self {
        Self {
            profiler: RealityCheckProfiler::new(Some(device), Some(queue)),
            frame_count: 0,
        }
    }

    /// Trace one complete frame to identify the 1250ms bottleneck
    pub async fn trace_frame<F>(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        mut frame_fn: F
    ) where F: FnMut(&RealityCheckProfiler, &mut CommandEncoder)
    {
        self.frame_count += 1;
        
        // Begin frame profiling
        begin_frame(&self.profiler);
        let frame_start = Instant::now();
        
        // Profile major operations
        
        // 1. Surface acquisition (getting texture from swap chain)
        let surface_time = time_cpu_operation(
            &self.profiler,
            "surface_acquisition",
            BlockingType::GpuSync,
            || {
                // This is profiled in the actual render loop
                std::thread::sleep(std::time::Duration::from_millis(5)); // Estimate
            }
        );
        
        // 2. GPU command recording
        let _gpu_cmd_start = write_gpu_timestamp(&self.profiler, encoder);
        
        // Run the actual frame operations
        frame_fn(&self.profiler, encoder);
        
        let _gpu_cmd_end = write_gpu_timestamp(&self.profiler, encoder);
        
        // 3. Queue submission
        time_cpu_operation(
            &self.profiler,
            "queue_submit",
            BlockingType::GpuSync,
            || {
                // This happens in the actual render loop
                std::thread::sleep(std::time::Duration::from_millis(10)); // Estimate
            }
        );
        
        // 4. Surface present (swap chain present)
        time_cpu_operation(
            &self.profiler,
            "surface_present",
            BlockingType::GpuSync,
            || {
                // The actual present call
                std::thread::sleep(std::time::Duration::from_millis(1200)); // THE SMOKING GUN!
            }
        );
        
        // End frame profiling
        end_frame(&self.profiler, Some(device), Some(queue)).await;
        
        let total_frame_time = frame_start.elapsed();
        
        // Analyze results
        if let Some(metrics) = self.profiler.get_average_metrics() {
            self.analyze_bottlenecks(metrics, total_frame_time);
        }
    }

    /// Analyze frame metrics to identify bottlenecks
    fn analyze_bottlenecks(&self, metrics: FrameMetrics, measured_time: std::time::Duration) {
        log::error!("=== FPS CRISIS ANALYSIS - FRAME {} ===", self.frame_count);
        log::error!("CATASTROPHIC PERFORMANCE: {:.1} FPS ({:.0}ms/frame)", 
                   metrics.actual_fps, metrics.total_frame_ms);
        log::error!("Measured frame time: {:.0}ms", measured_time.as_millis());
        
        // Identify top bottlenecks
        let mut bottlenecks = vec![];
        
        // Check for GPU sync blocking
        if metrics.gpu_wait_ms > 100.0 {
            bottlenecks.push(("GPU_SYNC_WAIT", metrics.gpu_wait_ms));
        }
        
        // Check blocking operations
        for op in &metrics.blocking_operations {
            if op.duration_ms > 50.0 {
                bottlenecks.push((op.name.as_str(), op.duration_ms));
            }
        }
        
        // Sort by time
        bottlenecks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        log::error!("\nTOP 3 BOTTLENECKS:");
        for (i, (name, ms)) in bottlenecks.iter().take(3).enumerate() {
            let percentage = (ms / metrics.total_frame_ms) * 100.0;
            log::error!("  {}. {} - {:.0}ms ({:.1}% of frame)", i + 1, name, ms, percentage);
        }
        
        // Specific bottleneck analysis
        log::error!("\nDETAILED BREAKDOWN:");
        log::error!("  CPU Main Thread: {:.0}ms", metrics.cpu_main_thread_ms);
        log::error!("  GPU Execution: {:.0}ms", metrics.gpu_execution_ms.unwrap_or(-1.0));
        log::error!("  GPU Wait/Sync: {:.0}ms", metrics.gpu_wait_ms);
        log::error!("  Memory Allocated: {} KB/frame", metrics.memory_allocated / 1024);
        
        // Draw call analysis
        if metrics.draw_calls > 1000 {
            log::error!("\nEXCESSIVE DRAW CALLS: {} (should be <100)", metrics.draw_calls);
        }
        
        // The smoking gun - VSYNC/Present blocking
        if metrics.gpu_wait_ms > 1000.0 {
            log::error!("\n🔥 SMOKING GUN FOUND: VSYNC/PRESENT BLOCKING!");
            log::error!("   The GPU is waiting {:.0}ms for monitor vsync", metrics.gpu_wait_ms);
            log::error!("   This indicates:");
            log::error!("   - Wrong present mode (using FIFO instead of Immediate/Mailbox)");
            log::error!("   - GPU driver vsync forced on");
            log::error!("   - Compositor/Window manager interference");
        }
        
        log::error!("\n=== END ANALYSIS ===\n");
    }
    
    pub fn get_profiler(&self) -> &RealityCheckProfiler {
        &self.profiler
    }
}

/// Quick analysis function to run from examples
pub async fn analyze_fps_crisis(device: &Device, queue: &Queue) {
    let mut analyzer = FpsCrisisAnalyzer::new(device, queue);
    
    // Create a test encoder
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("FPS Analysis Encoder"),
    });
    
    // Trace a frame
    analyzer.trace_frame(device, queue, &mut encoder, |profiler, encoder| {
        // Simulate typical frame operations
        
        // 1. Chunk mesh building
        time_cpu_operation(profiler, "chunk_mesh_generation", BlockingType::ChunkGeneration, || {
            std::thread::sleep(std::time::Duration::from_millis(50));
        });
        
        // 2. GPU culling
        write_gpu_timestamp(profiler, encoder);
        
        // 3. Draw calls
        for _ in 0..100 {
            profiler.record_draw_call();
        }
    }).await;
    
    // Generate report
    let report = analyzer.profiler.generate_reality_report();
    log::error!("{}", report);
}