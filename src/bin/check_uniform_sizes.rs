// Check sizes of various uniform structs
fn main() {
    // Check gpu_state CameraUniform size
    println!("=== gpu_state::CameraUniform ===");
    println!("Size: {} bytes", std::mem::size_of::<hearth_engine::renderer::CameraUniform>());
    println!("Expected: 208 bytes");
    println!("Breakdown:");
    println!("  view: 64 bytes");
    println!("  projection: 64 bytes");
    println!("  view_proj: 64 bytes");
    println!("  position: 12 bytes");
    println!("  _padding: 4 bytes");
    println!("  Total: 208 bytes");
    
    println!("\n=== gpu_driven::CameraData ===");
    println!("Size: {} bytes", std::mem::size_of::<hearth_engine::renderer::gpu_driven::culling_pipeline::CameraData>());
    println!("Expected: 208 bytes");
    
    println!("\n=== data_camera::CameraUniform ===");
    println!("Size: {} bytes", std::mem::size_of::<hearth_engine::camera::CameraUniform>());
    println!("Expected: 208 bytes");
}