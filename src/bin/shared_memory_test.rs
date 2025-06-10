use earth_engine::renderer::GpuState;
use std::time::Instant;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TestData {
    value: f32,
}

/// Test shared memory optimization in compute shaders
fn main() {
    println!("Shared Memory Optimization Test");
    println!("==============================\n");
    
    // Initialize GPU
    let gpu_state = pollster::block_on(GpuState::new(None)).expect("Failed to create GPU state");
    
    const DATA_SIZE: usize = 1024 * 1024; // 1M elements
    const WORKGROUP_SIZE: usize = 256;
    
    // Create test data
    let mut input_data = vec![TestData { value: 0.0 }; DATA_SIZE];
    for (i, item) in input_data.iter_mut().enumerate() {
        item.value = i as f32;
    }
    
    // Create buffers
    let input_buffer = gpu_state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Input Buffer"),
        contents: bytemuck::cast_slice(&input_data),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    
    let output_buffer = gpu_state.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: (DATA_SIZE * std::mem::size_of::<TestData>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    
    // Test 1: Naive shader (no shared memory)
    let naive_shader = gpu_state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Naive Shader"),
        source: wgpu::ShaderSource::Wgsl(r#"
            struct TestData {
                value: f32,
            }
            
            @group(0) @binding(0) var<storage, read> input: array<TestData>;
            @group(0) @binding(1) var<storage, read_write> output: array<TestData>;
            
            @compute @workgroup_size(256)
            fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
                let idx = global_id.x;
                if (idx >= arrayLength(&input)) {
                    return;
                }
                
                // Simulate neighbor access (27 reads per thread)
                var sum = 0.0;
                for (var i = 0u; i < 27u; i++) {
                    let neighbor_idx = (idx + i) % arrayLength(&input);
                    sum += input[neighbor_idx].value;
                }
                
                output[idx].value = sum / 27.0;
            }
        "#.into()),
    });
    
    // Test 2: Optimized shader (with shared memory)
    let optimized_shader = gpu_state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Optimized Shader"),
        source: wgpu::ShaderSource::Wgsl(r#"
            struct TestData {
                value: f32,
            }
            
            @group(0) @binding(0) var<storage, read> input: array<TestData>;
            @group(0) @binding(1) var<storage, read_write> output: array<TestData>;
            
            var<workgroup> shared_data: array<f32, 283>; // 256 + 27 for halo
            
            @compute @workgroup_size(256)
            fn main(
                @builtin(global_invocation_id) global_id: vec3<u32>,
                @builtin(local_invocation_id) local_id: vec3<u32>,
                @builtin(workgroup_id) workgroup_id: vec3<u32>
            ) {
                let local_idx = local_id.x;
                let global_idx = global_id.x;
                let workgroup_offset = workgroup_id.x * 256u;
                
                if (global_idx >= arrayLength(&input)) {
                    return;
                }
                
                // Load data into shared memory (including halo)
                shared_data[local_idx] = input[global_idx].value;
                
                // Load halo region
                if (local_idx < 27u) {
                    let halo_idx = workgroup_offset + 256u + local_idx;
                    if (halo_idx < arrayLength(&input)) {
                        shared_data[256u + local_idx] = input[halo_idx].value;
                    }
                }
                
                workgroupBarrier();
                
                // Now access from shared memory (much faster)
                var sum = 0.0;
                for (var i = 0u; i < 27u; i++) {
                    sum += shared_data[local_idx + i];
                }
                
                output[global_idx].value = sum / 27.0;
            }
        "#.into()),
    });
    
    // Create pipelines
    let bind_group_layout = gpu_state.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Test Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    
    let pipeline_layout = gpu_state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Test Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    
    let naive_pipeline = gpu_state.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Naive Pipeline"),
        layout: Some(&pipeline_layout),
        module: &naive_shader,
        entry_point: "main",
    });
    
    let optimized_pipeline = gpu_state.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Optimized Pipeline"),
        layout: Some(&pipeline_layout),
        module: &optimized_shader,
        entry_point: "main",
    });
    
    // Create bind group
    let bind_group = gpu_state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Test Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    });
    
    println!("Testing with {} elements", DATA_SIZE);
    println!("Workgroup size: {}", WORKGROUP_SIZE);
    println!("Each thread reads 27 neighbors\n");
    
    // Test naive implementation
    let mut total_naive_time = std::time::Duration::ZERO;
    const ITERATIONS: u32 = 10;
    
    for i in 0..ITERATIONS {
        let start = Instant::now();
        
        let mut encoder = gpu_state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Naive Encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Naive Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&naive_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups((DATA_SIZE as u32 + 255) / 256, 1, 1);
        }
        
        gpu_state.queue.submit(Some(encoder.finish()));
        gpu_state.device.poll(wgpu::Maintain::Wait);
        
        let elapsed = start.elapsed();
        if i > 0 { // Skip first iteration (warmup)
            total_naive_time += elapsed;
        }
    }
    
    let avg_naive_time = total_naive_time / (ITERATIONS - 1);
    println!("Naive implementation: {:?}", avg_naive_time);
    
    // Test optimized implementation
    let mut total_optimized_time = std::time::Duration::ZERO;
    
    for i in 0..ITERATIONS {
        let start = Instant::now();
        
        let mut encoder = gpu_state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Optimized Encoder"),
        });
        
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Optimized Pass"),
                timestamp_writes: None,
            });
            
            compute_pass.set_pipeline(&optimized_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups((DATA_SIZE as u32 + 255) / 256, 1, 1);
        }
        
        gpu_state.queue.submit(Some(encoder.finish()));
        gpu_state.device.poll(wgpu::Maintain::Wait);
        
        let elapsed = start.elapsed();
        if i > 0 { // Skip first iteration (warmup)
            total_optimized_time += elapsed;
        }
    }
    
    let avg_optimized_time = total_optimized_time / (ITERATIONS - 1);
    println!("Optimized implementation: {:?}", avg_optimized_time);
    
    let speedup = avg_naive_time.as_secs_f64() / avg_optimized_time.as_secs_f64();
    println!("\nSpeedup: {:.2}x", speedup);
    
    // Calculate memory bandwidth
    let total_reads_naive = DATA_SIZE * 27; // Each thread reads 27 values
    let total_writes = DATA_SIZE;
    let total_bytes = (total_reads_naive + total_writes) * 4; // 4 bytes per f32
    let bandwidth_naive = total_bytes as f64 / avg_naive_time.as_secs_f64() / (1024.0 * 1024.0 * 1024.0);
    
    let total_reads_optimized = DATA_SIZE + WORKGROUP_SIZE * 27; // Shared memory loads
    let total_bytes_opt = (total_reads_optimized + total_writes) * 4;
    let bandwidth_optimized = total_bytes_opt as f64 / avg_optimized_time.as_secs_f64() / (1024.0 * 1024.0 * 1024.0);
    
    println!("\nMemory Bandwidth:");
    println!("Naive: {:.2} GB/s", bandwidth_naive);
    println!("Optimized: {:.2} GB/s (effective)", bandwidth_optimized);
    
    println!("\nConclusion:");
    println!("Shared memory optimization provides ~{:.1}x speedup for neighbor access patterns", speedup);
    println!("This is critical for fluid simulation, SDF generation, and other spatial algorithms");
}