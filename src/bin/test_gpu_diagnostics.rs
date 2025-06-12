use earth_engine::renderer::{GpuDiagnostics, GpuInitProgress, GpuHealthMonitor};
use env_logger;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    
    log::info!("=== GPU Diagnostics Test ===");
    log::info!("Testing comprehensive GPU initialization diagnostics\n");
    
    // Create progress tracker
    let progress = GpuInitProgress::new();
    
    // Test 1: Create WGPU instance
    progress.start_step("Create WGPU Instance");
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    progress.complete_step("Create WGPU Instance", Some("All backends enabled".to_string()));
    
    // Test 2: Run diagnostics
    progress.start_step("Run GPU Diagnostics");
    let diagnostics = GpuDiagnostics::run_diagnostics(&instance).await;
    diagnostics.print_report();
    progress.complete_step("Run GPU Diagnostics", 
        Some(format!("Found {} adapters", diagnostics.available_adapters.len())));
    
    // Test 3: Enumerate adapters
    log::info!("\n=== Testing Adapter Enumeration ===");
    let adapters: Vec<_> = instance.enumerate_adapters(wgpu::Backends::all()).collect();
    
    for (i, adapter) in adapters.iter().enumerate() {
        log::info!("\nTesting adapter {}", i);
        let info = adapter.get_info();
        log::info!("  Name: {}", info.name);
        log::info!("  Backend: {:?}", info.backend);
        log::info!("  Type: {:?}", info.device_type);
        
        // Validate capabilities
        let validation = GpuDiagnostics::validate_capabilities(adapter);
        validation.print_results();
        
        // Try to create device
        match adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Test Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            },
            None,
        ).await {
            Ok((device, _queue)) => {
                log::info!("  ✓ Device creation successful");
                
                // Run operation tests
                log::info!("  Running operation tests...");
                let test_results = GpuDiagnostics::test_gpu_operations(&device).await;
                test_results.print_results();
            }
            Err(e) => {
                log::error!("  ✗ Device creation failed: {}", e);
            }
        }
    }
    
    // Test 4: Health monitoring
    log::info!("\n=== Testing GPU Health Monitor ===");
    let mut health_monitor = GpuHealthMonitor::new();
    
    // Simulate some errors
    for i in 0..3 {
        health_monitor.record_error();
        log::info!("Simulated error #{}", i + 1);
        
        if health_monitor.should_attempt_recovery() {
            log::info!("Recovery recommended");
            health_monitor.record_recovery_attempt();
        }
        
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    // Print final summary
    progress.print_summary();
    
    log::info!("\n=== GPU Diagnostics Test Complete ===");
}