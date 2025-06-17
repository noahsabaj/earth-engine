/// Test to verify GPU terrain generation produces proper terrain instead of vertical strips
use hearth_engine::*;
use hearth_engine::world::{WorldGenerator, ChunkPos};
use hearth_engine::world::generation::{GpuDefaultWorldGenerator, DefaultWorldGenerator};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    println!("\n=== Testing GPU Terrain Generation for Vertical Strips ===\n");
    
    // Initialize GPU
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
        .expect("Failed to find adapter");
    
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
    
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    
    // Create GPU world generator
    let gpu_generator = GpuDefaultWorldGenerator::new(
        device.clone(),
        queue.clone(),
        12345, // seed
        BlockId::GRASS,
        BlockId::DIRT,
        BlockId::STONE,
        BlockId::WATER,
        BlockId::SAND,
    );
    
    // Create CPU world generator for comparison
    let cpu_generator = DefaultWorldGenerator::new(
        12345, // same seed
        BlockId::GRASS,
        BlockId::DIRT,
        BlockId::STONE,
        BlockId::WATER,
        BlockId::SAND,
    );
    
    // Test several chunks at ground level
    let test_positions = vec![
        ChunkPos::new(0, 0, 0),
        ChunkPos::new(1, 0, 0),
        ChunkPos::new(0, 1, 0),
        ChunkPos::new(0, 2, 0),
    ];
    
    println!("Testing chunks for vertical strip patterns...\n");
    
    for pos in test_positions {
        println!("Generating chunk at {:?}:", pos);
        
        // Generate with CPU
        let cpu_chunk = cpu_generator.generate_chunk(pos, 32);
        let mut cpu_layers = vec![0; 32]; // Count blocks per Y layer
        
        for y in 0..32 {
            for x in 0..32 {
                for z in 0..32 {
                    if cpu_chunk.get_block(x, y, z) != BlockId::AIR {
                        cpu_layers[y as usize] += 1;
                    }
                }
            }
        }
        
        // Generate with GPU
        let gpu_chunk = gpu_generator.generate_chunk(pos, 32);
        let mut gpu_layers = vec![0; 32]; // Count blocks per Y layer
        
        for y in 0..32 {
            for x in 0..32 {
                for z in 0..32 {
                    if gpu_chunk.get_block(x, y, z) != BlockId::AIR {
                        gpu_layers[y as usize] += 1;
                    }
                }
            }
        }
        
        // Analyze patterns
        println!("  CPU terrain distribution by Y layer:");
        for (y, count) in cpu_layers.iter().enumerate() {
            if *count > 0 {
                println!("    Y={:2}: {} blocks ({:4.1}% of layer)", 
                    y, count, (*count as f32 / 1024.0) * 100.0);
            }
        }
        
        println!("  GPU terrain distribution by Y layer:");
        let mut strip_detected = false;
        for (y, count) in gpu_layers.iter().enumerate() {
            if *count > 0 {
                println!("    Y={:2}: {} blocks ({:4.1}% of layer)", 
                    y, count, (*count as f32 / 1024.0) * 100.0);
                
                // Check for vertical strip pattern (all blocks in top layers only)
                if y >= 24 && *count == 1024 {
                    strip_detected = true;
                }
            }
        }
        
        if strip_detected {
            println!("  ⚠️  WARNING: Vertical strip pattern detected! Top layers are completely filled.");
        } else {
            println!("  ✅ Good: Natural terrain variation detected");
        }
        
        // Compare total blocks
        let cpu_total: u32 = cpu_layers.iter().sum();
        let gpu_total: u32 = gpu_layers.iter().sum();
        println!("  Total blocks: CPU={}, GPU={}", cpu_total, gpu_total);
        println!();
    }
    
    println!("=== Test Complete ===");
}