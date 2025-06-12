use earth_engine::renderer::{GpuDiagnostics, OperationTestResult};
use env_logger;
use log;

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    log::info!("Starting GPU diagnostics test...");

    // Create WGPU instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    // Run diagnostics
    let report = GpuDiagnostics::run_diagnostics(&instance).await;
    report.print_report();

    // Get an adapter
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await;

    if let Some(adapter) = adapter {
        log::info!("\nValidating GPU capabilities...");
        let validation = GpuDiagnostics::validate_capabilities(&adapter);
        validation.print_results();

        // Create device to test operations
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Test Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        log::info!("\nTesting GPU operations...");
        let test_result = GpuDiagnostics::test_gpu_operations(&device).await;
        test_result.print_results();
    } else {
        log::error!("No suitable GPU adapter found!");
    }

    log::info!("\nGPU diagnostics test complete.");
}