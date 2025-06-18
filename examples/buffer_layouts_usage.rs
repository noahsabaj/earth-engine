//! Example demonstrating the use of centralized GPU buffer layouts

use std::sync::Arc;
use hearth_engine::gpu::buffer_layouts::{
    self, VoxelData, InstanceData, IndirectDrawCommand, CameraUniform,
    bindings, calculations, constants::*, layouts, usage,
};
use cgmath::{Matrix4, Vector3, SquareMatrix};
use wgpu::util::DeviceExt;

fn main() {
    // This example shows how to use the centralized buffer layout system
    pollster::block_on(run());
}

async fn run() {
    // Initialize WGPU
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();
    
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .unwrap();
    
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    
    println!("=== Centralized GPU Buffer Layouts Example ===\n");
    
    // Example 1: World Buffer with proper layout
    example_world_buffer(&device, &queue);
    
    // Example 2: Instance Buffer with centralized calculations
    example_instance_buffer(&device, &queue);
    
    // Example 3: Camera Uniform with proper alignment
    example_camera_uniform(&device, &queue);
    
    // Example 4: Indirect Commands for GPU-driven rendering
    example_indirect_commands(&device, &queue);
    
    // Example 5: Using centralized bind group layouts
    example_bind_groups(&device);
    
    println!("\n=== All examples completed successfully! ===");
}

fn example_world_buffer(device: &wgpu::Device, queue: &wgpu::Queue) {
    println!("1. World Buffer Example:");
    
    // Use centralized layout calculation
    let layout = buffer_layouts::WorldBufferLayout::new(3); // view distance = 3
    
    println!("   - View distance: {}", layout.view_distance);
    println!("   - Max chunks: {}", layout.max_chunks);
    println!("   - Voxel buffer size: {} MB", layout.voxel_buffer_size / (1024 * 1024));
    println!("   - Total memory: {:.2} MB", layout.memory_usage_mb());
    
    // Create voxel buffer using centralized usage flags
    let voxel_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("World Voxel Buffer"),
        size: layout.voxel_buffer_size,
        usage: usage::STORAGE_READ, // Centralized usage flags
        mapped_at_creation: false,
    });
    
    // Example: Calculate chunk offset using centralized function
    let chunk_slot = 42;
    let offset = calculations::chunk_slot_offset(chunk_slot);
    println!("   - Chunk slot {} starts at offset {} bytes", chunk_slot, offset);
    
    // Example voxel data using centralized type
    let voxel = VoxelData::new(1, 15, 10, 0); // stone, full light, some skylight
    println!("   - Created voxel: block_id={}, light={}, sky_light={}", 
             voxel.block_id(), voxel.light_level(), voxel.sky_light_level());
}

fn example_instance_buffer(device: &wgpu::Device, queue: &wgpu::Queue) {
    println!("\n2. Instance Buffer Example:");
    
    // Use centralized capacity presets
    let capacity = buffer_layouts::instance::presets::MEDIUM_CAPACITY;
    let buffer_size = calculations::instance_buffer_size(capacity);
    
    println!("   - Capacity: {} instances", capacity);
    println!("   - Buffer size: {} MB", buffer_size / (1024 * 1024));
    println!("   - Instance size: {} bytes", INSTANCE_DATA_SIZE);
    
    // Create instances using centralized type
    let instances = vec![
        InstanceData::new(Vector3::new(0.0, 0.0, 0.0), 1.0, [1.0, 0.0, 0.0, 1.0]),
        InstanceData::new(Vector3::new(5.0, 0.0, 0.0), 1.5, [0.0, 1.0, 0.0, 1.0]),
        InstanceData::new(Vector3::new(10.0, 0.0, 0.0), 2.0, [0.0, 0.0, 1.0, 1.0]),
    ];
    
    // Create buffer with instances
    let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Instance Buffer"),
        contents: bytemuck::cast_slice(&instances),
        usage: usage::VERTEX | usage::STORAGE,
    });
    
    println!("   - Created {} instances", instances.len());
    
    // Show vertex buffer layout
    let layout = buffer_layouts::InstanceBufferLayout::vertex_layout();
    println!("   - Vertex stride: {} bytes", layout.array_stride);
}

fn example_camera_uniform(device: &wgpu::Device, queue: &wgpu::Queue) {
    println!("\n3. Camera Uniform Example:");
    
    // Create camera matrices
    let view = Matrix4::look_at_rh(
        cgmath::Point3::new(0.0, 5.0, 10.0),
        cgmath::Point3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    
    let projection = cgmath::perspective(
        cgmath::Deg(60.0),
        1920.0 / 1080.0,
        0.1,
        1000.0,
    );
    
    // Create camera uniform using centralized type
    let mut camera_uniform = CameraUniform::new(
        view,
        projection,
        Vector3::new(0.0, 5.0, 10.0),
        Vector3::new(0.0, -0.5, -0.866),
        0.1,
        1000.0,
        1920.0,
        1080.0,
    );
    
    camera_uniform.update_time(1.234, 0.016);
    
    // Create uniform buffer with proper alignment
    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Uniform"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: usage::UNIFORM,
    });
    
    println!("   - Camera buffer size: {} bytes (aligned)", CAMERA_UNIFORM_SIZE);
    println!("   - Position: {:?}", camera_uniform.position);
    println!("   - Near/Far: {:?}", camera_uniform.near_far);
}

fn example_indirect_commands(device: &wgpu::Device, queue: &wgpu::Queue) {
    println!("\n4. Indirect Commands Example:");
    
    // Create indirect draw commands using centralized types
    let commands = vec![
        IndirectDrawCommand::new(36, 100),     // 36 vertices, 100 instances
        IndirectDrawCommand::new(24, 50),      // 24 vertices, 50 instances
        IndirectDrawCommand::with_offsets(12, 25, 100, 150), // with offsets
    ];
    
    let buffer_size = calculations::indirect_buffer_size(commands.len() as u32, false);
    
    // Create indirect buffer
    let indirect_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Indirect Commands"),
        contents: bytemuck::cast_slice(&commands),
        usage: usage::INDIRECT,
    });
    
    println!("   - Created {} indirect commands", commands.len());
    println!("   - Command size: {} bytes", INDIRECT_COMMAND_SIZE);
    println!("   - Buffer size: {} bytes", buffer_size);
}

fn example_bind_groups(device: &wgpu::Device) {
    println!("\n5. Bind Group Layouts Example:");
    
    // Create bind group layout using centralized helpers
    let world_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("World Buffer Layout"),
        entries: &[
            layouts::storage_buffer_entry(
                bindings::world::VOXEL_BUFFER,
                false,
                wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::FRAGMENT,
            ),
            layouts::storage_buffer_entry(
                bindings::world::METADATA_BUFFER,
                true,
                wgpu::ShaderStages::COMPUTE,
            ),
            layouts::uniform_buffer_entry(
                bindings::world::PARAMS_BUFFER,
                wgpu::ShaderStages::COMPUTE,
            ),
        ],
    });
    
    println!("   - Created world buffer layout with bindings:");
    println!("     - Voxel buffer: binding {}", bindings::world::VOXEL_BUFFER);
    println!("     - Metadata buffer: binding {}", bindings::world::METADATA_BUFFER);
    println!("     - Params buffer: binding {}", bindings::world::PARAMS_BUFFER);
    
    // Create render bind group layout
    let render_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Render Layout"),
        entries: &[
            layouts::uniform_buffer_entry(
                bindings::render::CAMERA_UNIFORM,
                wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ),
            layouts::storage_buffer_entry(
                bindings::render::INSTANCE_BUFFER,
                true,
                wgpu::ShaderStages::VERTEX,
            ),
        ],
    });
    
    println!("   - Created render layout with bindings:");
    println!("     - Camera uniform: binding {}", bindings::render::CAMERA_UNIFORM);
    println!("     - Instance buffer: binding {}", bindings::render::INSTANCE_BUFFER);
    
    // Show memory budget calculations
    println!("\n   Memory Budget Calculations:");
    let chunks_in_budget = chunks_per_memory_budget(WORLD_BUFFER_MEMORY_BUDGET_MB);
    println!("     - Chunks in {} MB budget: {}", WORLD_BUFFER_MEMORY_BUDGET_MB, chunks_in_budget);
    
    let recommended_view = recommended_view_distance(512); // 512 MB available
    println!("     - Recommended view distance for 512 MB: {}", recommended_view);
}