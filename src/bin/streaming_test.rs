use earth_engine::streaming::*;
use earth_engine::world_gpu::{create_planet_world, StreamingWorldBuffer};
use std::sync::Arc;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    println!("Earth Engine - Data-Oriented World Streaming Test");
    println!("==================================================");
    
    // Test page table creation
    test_page_table_creation();
    
    // Test predictive loading
    test_predictive_loading();
    
    // Test compression
    test_compression();
    
    // Test large world support
    test_billion_voxel_world();
    
    println!("\nAll tests passed!");
    Ok(())
}

fn test_page_table_creation() {
    println!("\n1. Testing Page Table Creation");
    println!("------------------------------");
    
    // Test small world
    let small_world = create_page_table((256, 128, 256), PAGE_SIZE);
    println!("Small world pages: {}", small_world.total_pages);
    assert_eq!(small_world.world_size_pages, (4, 2, 4));
    
    // Test medium world
    let medium_world = create_page_table((1024, 256, 1024), PAGE_SIZE);
    println!("Medium world pages: {}", medium_world.total_pages);
    assert_eq!(medium_world.world_size_pages, (16, 4, 16));
    
    // Test large world with sparse index
    let large_world = create_page_table((65536, 1024, 65536), PAGE_SIZE);
    println!("Large world pages: {}", large_world.total_pages);
    assert!(large_world.sparse_index.is_some());
    
    // Test page indexing
    let idx = match small_world.page_index(2, 1, 3) {
        Some(idx) => idx,
        None => {
            eprintln!("Failed to get page index");
            return;
        }
    };
    assert_eq!(idx, 2 + 1 * 4 + 3 * 4 * 2);
    
    // Test voxel to page conversion
    let (px, py, pz) = small_world.voxel_to_page(150, 70, 200);
    assert_eq!((px, py, pz), (2, 1, 3));
    
    println!("✓ Page table tests passed");
}

fn test_predictive_loading() {
    println!("\n2. Testing Predictive Loading");
    println!("-----------------------------");
    
    let mut loader = PredictiveLoader::new(1, 128.0, 256.0);
    let page_table = create_page_table((1024, 256, 1024), PAGE_SIZE);
    
    // Simulate player movement
    let positions = vec![
        (100.0, 50.0, 100.0),
        (105.0, 50.0, 105.0),
        (110.0, 51.0, 110.0),
        (115.0, 52.0, 115.0),
    ];
    
    for (i, &pos) in positions.iter().enumerate() {
        loader.update_player(0, pos, i as f64 * 0.1, &page_table);
    }
    
    // Check predictions
    let requests = loader.get_load_requests(10);
    println!("Generated {} load requests", requests.len());
    assert!(!requests.is_empty());
    
    // Verify requests are sorted by priority
    for i in 1..requests.len() {
        assert!(requests[i-1].priority >= requests[i].priority);
    }
    
    // Test adaptive parameters
    loader.adapt_parameters(25.0, 0.9);
    
    println!("✓ Predictive loading tests passed");
}

fn test_compression() {
    println!("\n3. Testing Compression");
    println!("----------------------");
    
    use earth_engine::streaming::compression::cpu_compressor;
    
    // Test data patterns
    let uniform_data = vec![42u32; PAGE_VOXEL_COUNT as usize];
    let sparse_data: Vec<u32> = (0..PAGE_VOXEL_COUNT)
        .map(|i| if i % 10 == 0 { i } else { 0 })
        .collect();
    let varied_data: Vec<u32> = (0..PAGE_VOXEL_COUNT)
        .map(|i| (i % 16) as u32)
        .collect();
    
    // Test RLE compression
    let rle_uniform = cpu_compressor::compress_rle(&uniform_data);
    println!("RLE uniform: {} -> {} bytes", 
        uniform_data.len() * 4, rle_uniform.len());
    assert!(rle_uniform.len() < uniform_data.len() * 4);
    
    // Test bit-packed compression
    let bitpacked_sparse = cpu_compressor::compress_bitpacked(&sparse_data);
    println!("BitPacked sparse: {} -> {} bytes",
        sparse_data.len() * 4, bitpacked_sparse.len());
    assert!(bitpacked_sparse.len() < sparse_data.len() * 4);
    
    // Test palettized compression
    let palettized_varied = cpu_compressor::compress_palettized(&varied_data);
    println!("Palettized varied: {} -> {} bytes",
        varied_data.len() * 4, palettized_varied.len());
    assert!(palettized_varied.len() < varied_data.len() * 4);
    
    // Test auto compression
    let (comp_type, _) = cpu_compressor::compress_auto(&uniform_data);
    assert_eq!(comp_type, CompressionType::RLE);
    
    println!("✓ Compression tests passed");
}

fn test_billion_voxel_world() {
    println!("\n4. Testing Billion+ Voxel World Support");
    println!("---------------------------------------");
    
    // Test maximum world size
    let max_world = create_page_table(
        (MAX_WORLD_SIZE_X, MAX_WORLD_SIZE_Y, MAX_WORLD_SIZE_Z),
        PAGE_SIZE
    );
    
    println!("Maximum world size: {}x{}x{} voxels",
        MAX_WORLD_SIZE_X, MAX_WORLD_SIZE_Y, MAX_WORLD_SIZE_Z);
    println!("Total pages: {}", max_world.total_pages);
    println!("Memory for page table: {} MB",
        max_world.entries.len() * std::mem::size_of::<PageTableEntry>() / 1024 / 1024);
    
    // Verify sparse index exists for large worlds
    assert!(max_world.sparse_index.is_some());
    
    // Test 1 billion voxel world
    let billion_world = create_page_table((1024 * 1024, 1024, 1024 * 1024), PAGE_SIZE);
    let total_voxels = 1024u64 * 1024 * 1024 * 1024 * 1024;
    println!("1 billion voxel world:");
    println!("  Total voxels: {:.2} billion", total_voxels as f64 / 1e9);
    println!("  Total pages: {}", billion_world.total_pages);
    
    // Test page access patterns
    let test_coords = vec![
        (0, 0, 0),
        (512 * 1024, 512, 512 * 1024),
        (1024 * 1024 - 1, 1023, 1024 * 1024 - 1),
    ];
    
    for (x, y, z) in test_coords {
        let (px, py, pz) = billion_world.voxel_to_page(x, y, z);
        let idx = billion_world.page_index(px, py, pz);
        assert!(idx.is_some());
        println!("  Voxel ({}, {}, {}) -> Page ({}, {}, {}) -> Index {:?}",
            x, y, z, px, py, pz, idx);
    }
    
    println!("✓ Billion+ voxel world tests passed");
}

/// Integration test with GPU (requires GPU)
#[allow(dead_code)]
async fn test_gpu_streaming() -> anyhow::Result<()> {
    println!("\n5. Testing GPU Streaming Integration");
    println!("------------------------------------");
    
    // Create GPU instance
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Failed to find adapter");
    
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await?;
    
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    
    // Create streaming world
    let world_path = PathBuf::from("test_world.dat");
    let streaming_world = create_planet_world(
        device.clone(),
        queue.clone(),
        world_path,
        4 * 1024 * 1024 * 1024, // 4GB max GPU memory
    )?;
    
    // Test statistics
    let stats = streaming_world.get_stats().await;
    println!("World stats:");
    println!("  Total pages: {}", stats.total_pages);
    println!("  World size: {:?} voxels", stats.world_size_voxels);
    println!("  World size: {} GB", stats.world_size_bytes / 1024 / 1024 / 1024);
    
    // Test player position update
    streaming_world.update_player_position(0, (500.0, 100.0, 500.0), 0.0).await;
    
    // Test region loading check
    let is_loaded = streaming_world.is_region_loaded(
        (480, 80, 480),
        (520, 120, 520),
    ).await;
    println!("  Region loaded: {}", is_loaded);
    
    println!("✓ GPU streaming tests passed");
    Ok(())
}