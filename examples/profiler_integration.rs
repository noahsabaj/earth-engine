//! Example showing how to integrate RealityCheckProfiler into existing code
//! 
//! This demonstrates the minimal changes needed to add reality profiling
//! to an existing Hearth Engine application.

use earth_engine::profiling::{
    RealityCheckProfiler, BlockingType, SystemMetrics,
    time_cpu_operation, write_gpu_timestamp,
};

/// Example of integrating profiler into a render loop
pub struct ProfiledRenderer {
    profiler: RealityCheckProfiler,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl ProfiledRenderer {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let mut profiler = RealityCheckProfiler::new(Some(&device), Some(&queue));
        // In a real app, set up memory tracking early in main()
        
        Self {
            profiler,
            device,
            queue,
        }
    }
    
    /// Example render frame with profiling
    pub async fn render_frame(&mut self) {
        use earth_engine::profiling::{
            reality_begin_frame, reality_end_frame,
            record_draw_call, record_compute_dispatch,
            generate_reality_report,
        };
        
        // Start frame profiling
        reality_begin_frame(&self.profiler);
        
        // Profile chunk generation
        time_cpu_operation(&self.profiler, "chunk_generation", BlockingType::ChunkGeneration, || {
            // Your chunk generation code here
            std::thread::sleep(std::time::Duration::from_millis(50)); // Simulating slow chunk gen
        });
        
        // Profile mesh building
        time_cpu_operation(&self.profiler, "mesh_building", BlockingType::MeshBuilding, || {
            // Your mesh building code here
            std::thread::sleep(std::time::Duration::from_millis(20));
        });
        
        // Create command encoder with GPU timing
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Profiled Frame"),
        });
        
        // Start GPU timing
        let _gpu_start = write_gpu_timestamp(&self.profiler, &mut encoder);
        
        // Compute pass with profiling
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Physics Compute"),
            });
            
            // Record compute dispatches
            for _ in 0..10 {
                record_compute_dispatch(&self.profiler);
                // Your compute dispatch code here
            }
        }
        
        // Render pass with profiling
        time_cpu_operation(&self.profiler, "render_pass_setup", BlockingType::CpuWork, || {
            // Render pass setup code
        });
        
        // End GPU timing
        write_gpu_timestamp(&self.profiler, &mut encoder);
        
        // Submit with profiling
        time_cpu_operation(&self.profiler, "gpu_submit", BlockingType::GpuSync, || {
            self.queue.submit(Some(encoder.finish()));
        });
        
        // End frame profiling
        reality_end_frame(&self.profiler, Some(&self.device), Some(&self.queue)).await;
        
        // Record system-level metrics
        self.profiler.record_system_metrics("terrain_system", SystemMetrics {
            system_name: "terrain_system".to_string(),
            cpu_time_ms: 75.0,
            gpu_time_ms: Some(5.0),
            memory_allocated: 1024 * 1024 * 8, // 8MB
            is_blocking_main_thread: true,
        });
        
        // Generate report every N frames
        static mut FRAME_COUNT: u32 = 0;
        unsafe {
            FRAME_COUNT += 1;
            if FRAME_COUNT % 300 == 0 { // Every ~5 seconds at 60 FPS
                println!("{}", generate_reality_report(&self.profiler));
            }
        }
    }
}

/// Example of profiling a specific system
pub fn profile_physics_system(profiler: &RealityCheckProfiler) {
    // Time the entire physics update
    time_cpu_operation(profiler, "physics_total", BlockingType::PhysicsUpdate, || {
        // Broad phase
        time_cpu_operation(profiler, "physics_broad_phase", BlockingType::PhysicsUpdate, || {
            // Your broad phase code
            std::thread::sleep(std::time::Duration::from_millis(10));
        });
        
        // Narrow phase
        time_cpu_operation(profiler, "physics_narrow_phase", BlockingType::PhysicsUpdate, || {
            // Your narrow phase code
            std::thread::sleep(std::time::Duration::from_millis(15));
        });
        
        // Integration
        time_cpu_operation(profiler, "physics_integration", BlockingType::PhysicsUpdate, || {
            // Your integration code
            std::thread::sleep(std::time::Duration::from_millis(5));
        });
    });
    
    // Record the metrics
    profiler.record_system_metrics("physics", SystemMetrics {
        system_name: "physics".to_string(),
        cpu_time_ms: 30.0,
        gpu_time_ms: None,
        memory_allocated: 1024 * 512,
        is_blocking_main_thread: true,
    });
}

/// Example showing how to add profiling to existing code with minimal changes
pub fn minimal_integration_example() {
    // Before:
    /*
    fn update_world() {
        generate_chunks();
        update_physics();
        build_meshes();
    }
    */
    
    // After:
    fn update_world(profiler: &RealityCheckProfiler) {
        time_cpu_operation(profiler, "generate_chunks", BlockingType::ChunkGeneration, || {
            generate_chunks();
        });
        
        time_cpu_operation(profiler, "update_physics", BlockingType::PhysicsUpdate, || {
            update_physics();
        });
        
        time_cpu_operation(profiler, "build_meshes", BlockingType::MeshBuilding, || {
            build_meshes();
        });
    }
    
    // Stub functions for example
    fn generate_chunks() {}
    fn update_physics() {}
    fn build_meshes() {}
}

fn main() {
    println!("This is an example showing how to integrate the RealityCheckProfiler.");
    println!("See the code for integration patterns.");
}