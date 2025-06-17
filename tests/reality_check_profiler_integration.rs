/// Integration test for the Reality Check Profiler
use hearth_engine::profiling::{
    RealityCheckProfiler, BlockingType,
    reality_begin_frame, reality_end_frame, time_cpu_operation,
    record_draw_call, record_compute_dispatch, generate_reality_report,
};
use std::time::Duration;
use std::thread;

#[tokio::test]
async fn test_profiler_integration() {
    // Create profiler without GPU support for testing
    let profiler = RealityCheckProfiler::new(None, None);
    
    // Simulate a few frames
    for frame in 0..5 {
        reality_begin_frame(&profiler);
        
        // Simulate some CPU work
        let _result = time_cpu_operation(&profiler, "test_operation", BlockingType::CpuWork, || {
            thread::sleep(Duration::from_millis(2));
            42
        });
        
        // Record some draw calls
        for _ in 0..10 {
            record_draw_call(&profiler);
        }
        
        // Record some compute dispatches
        for _ in 0..5 {
            record_compute_dispatch(&profiler);
        }
        
        // End frame (no GPU devices in test)
        reality_end_frame(&profiler, None, None).await;
        
        // Small delay between frames
        thread::sleep(Duration::from_millis(10));
    }
    
    // Generate report
    let report = generate_reality_report(&profiler);
    
    // Verify report contains expected sections
    assert!(report.contains("EARTH ENGINE REALITY CHECK REPORT"));
    assert!(report.contains("ACTUAL PERFORMANCE"));
    assert!(report.contains("FRAME TIME BREAKDOWN"));
    assert!(report.contains("RENDERING STATS"));
    assert!(report.contains("Draw Calls"));
    assert!(report.contains("Compute Dispatches"));
    
    // Verify metrics were collected
    let metrics = profiler.get_average_metrics();
    assert!(metrics.is_some());
    
    let avg = metrics.unwrap();
    assert!(avg.draw_calls > 0);
    assert!(avg.compute_dispatches > 0);
    assert!(avg.total_frame_ms > 0.0);
    assert!(avg.actual_fps > 0.0);
}

#[test]
fn test_profiler_without_frames() {
    let profiler = RealityCheckProfiler::new(None, None);
    
    // Should handle no data gracefully
    let report = generate_reality_report(&profiler);
    assert!(report.contains("NO PERFORMANCE DATA COLLECTED YET"));
    
    let metrics = profiler.get_average_metrics();
    assert!(metrics.is_none());
}