//! Binary to run the voxel size impact analysis
//! 
//! This demonstrates why converting to 1dcm³ voxels would be catastrophic.

mod voxel_size_impact_analysis;
mod voxel_size_benchmark;

use std::sync::Arc;

#[tokio::main]
async fn main() {
    println!("\n🚨 EARTH ENGINE VOXEL SIZE IMPACT ANALYSIS 🚨\n");
    
    // Run the mathematical analysis
    voxel_size_impact_analysis::run_analysis();
    
    // Separator
    println!("\n");
    println!("════════════════════════════════════════════════════════════════════");
    println!("\n");
    
    // Try to run actual benchmarks (if GPU is available)
    match initialize_gpu().await {
        Ok((device, queue)) => {
            println!("GPU initialized, running performance benchmarks...\n");
            voxel_size_benchmark::run_voxel_size_benchmarks(device, queue).await;
        },
        Err(e) => {
            println!("⚠️  Could not initialize GPU: {}", e);
            println!("Skipping GPU benchmarks, but the analysis above shows the impact.");
        }
    }
    
    // Final message
    println!("\n");
    println!("════════════════════════════════════════════════════════════════════");
    println!("\n📊 ANALYSIS COMPLETE\n");
    println!("TL;DR: 1dcm³ voxels = 1000x more voxels = engine death 💀");
    println!("\nThe engine needs MASSIVE optimization before considering smaller voxels.");
}

async fn initialize_gpu() -> Result<(Arc<wgpu::Device>, Arc<wgpu::Queue>), String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .ok_or("Failed to find GPU adapter")?;
    
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Voxel Analysis Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )
        .await
        .map_err(|e| format!("Failed to create device: {}", e))?;
    
    Ok((Arc::new(device), Arc::new(queue)))
}