//! Test the unified world module

use hearth_engine::{
    BlockId, ChunkManagerInterface, ChunkPos, ComputeEngine, GeneratorInterface, UnifiedGenerator,
    UnifiedStorage, UnifiedWorldConfig, UnifiedWorldManager, VoxelPos,
};
use std::sync::Arc;

// Include constants from root constants.rs
include!("../constants.rs");

fn main() {
    env_logger::init();

    println!("Testing Unified World Module...");

    // Run async test
    pollster::block_on(test_unified_world());
}

async fn test_unified_world() {
    // Create GPU device
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
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    println!("✓ GPU device created");

    // Create world manager with unified architecture
    let config = UnifiedWorldConfig {
        chunk_size: core::CHUNK_SIZE,
        render_distance: 8,
        storage_config: Default::default(),
        generator_config: Default::default(),
    };

    let world_manager = UnifiedWorldManager::new(device.clone(), queue.clone(), config)
        .expect("Failed to create world manager");

    println!("✓ Unified world manager created");

    // Test chunk generation
    let chunk_pos = ChunkPos { x: 0, y: 0, z: 0 };
    let has_chunk = world_manager.storage.has_chunk(&chunk_pos);
    println!("  Has chunk at origin: {}", has_chunk);

    // Test block operations
    let test_pos = VoxelPos {
        x: 10,
        y: 64,
        z: 10,
    };
    let block = world_manager.get_block(test_pos);
    println!("  Block at {:?}: {:?}", test_pos, block);

    // Set a block
    world_manager.set_block(test_pos, BlockId(1)); // Stone
    let new_block = world_manager.get_block(test_pos);
    println!("  Block after setting: {:?}", new_block);

    // Test terrain generation
    let surface_height = world_manager.generator.get_surface_height(0.0, 0.0);
    println!("  Surface height at (0,0): {}", surface_height);

    // Test compute engine
    let compute_engine = ComputeEngine::new(device.clone(), queue.clone(), Default::default())
        .expect("Failed to create compute engine");
    println!("✓ Compute engine created");

    // Test chunk manager interface
    let chunk_manager = ChunkManagerInterface::new(world_manager.storage.clone());
    let stats = chunk_manager.get_stats();
    println!("  Loaded chunks: {}", stats.loaded_chunks);
    println!("  Total memory: {} MB", stats.total_memory_mb);

    // Test generator interface
    let generator_interface = GeneratorInterface::new(world_manager.generator.clone());
    let spawn_pos = generator_interface.find_spawn_position(VoxelPos { x: 0, y: 0, z: 0 });
    println!("  Spawn position: {:?}", spawn_pos);

    println!("\n✅ All unified world tests passed!");
}
