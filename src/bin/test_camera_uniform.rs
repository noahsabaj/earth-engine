use earth_engine::renderer::gpu_state::{CameraUniform, GpuCameraUniform};
use earth_engine::Camera;
use cgmath::{Point3, Deg, SquareMatrix};

fn main() {
    println!("Testing camera uniform sizes:");
    println!("CameraUniform size: {} bytes", std::mem::size_of::<CameraUniform>());
    println!("GpuCameraUniform size: {} bytes", std::mem::size_of::<GpuCameraUniform>());
    println!("Expected by shader: 64 bytes");
    
    // Create a test camera
    let mut camera = Camera::new(1920, 1080);
    camera.position = Point3::new(10.0, 20.0, 30.0);
    camera.yaw = Deg(-45.0);
    camera.pitch = Deg(15.0);
    
    // Create camera uniform and update it
    let mut camera_uniform = CameraUniform::new();
    camera_uniform.update_view_proj(&camera);
    
    // Convert to GPU uniform
    let gpu_uniform = camera_uniform.to_gpu_uniform();
    
    println!("\nCamera state:");
    println!("Position: {:?}", camera.position);
    println!("Yaw: {:?}, Pitch: {:?}", camera.yaw, camera.pitch);
    
    println!("\nView-Proj matrix:");
    for row in &gpu_uniform.view_proj {
        println!("[{:.3}, {:.3}, {:.3}, {:.3}]", row[0], row[1], row[2], row[3]);
    }
    
    // Verify the matrices
    let view = camera.build_view_matrix();
    let proj = camera.build_projection_matrix();
    let view_proj = proj * view;
    
    println!("\nVerifying matrix multiplication:");
    println!("View matrix determinant: {:.3}", view.determinant());
    println!("Proj matrix determinant: {:.3}", proj.determinant());
    println!("View-Proj matrix determinant: {:.3}", view_proj.determinant());
    
    // Check that the GPU uniform matches
    let gpu_view_proj: [[f32; 4]; 4] = view_proj.into();
    let matches = gpu_uniform.view_proj == gpu_view_proj;
    println!("\nGPU uniform matches calculated view_proj: {}", matches);
    
    if !matches {
        println!("ERROR: Matrices don't match!");
        for i in 0..4 {
            for j in 0..4 {
                if gpu_uniform.view_proj[i][j] != gpu_view_proj[i][j] {
                    println!("  Mismatch at [{},{}]: {} != {}", 
                        i, j, gpu_uniform.view_proj[i][j], gpu_view_proj[i][j]);
                }
            }
        }
    }
}