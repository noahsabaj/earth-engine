use hearth_engine::renderer::gpu_driven::culling_pipeline::CameraData;

fn main() {
    println!("Size of CameraData: {} bytes", std::mem::size_of::<CameraData>());
    println!("Expected size: 208 bytes");
    println!("view_proj: {} bytes", 64);
    println!("position: {} bytes", 12);
    println!("_padding0: {} bytes", 4);
    println!("frustum_planes: {} bytes", 96);
    println!("Total without padding: {} bytes", 64 + 12 + 4 + 96);
    
    // Also check alignment
    println!("\nAlignment of CameraData: {} bytes", std::mem::align_of::<CameraData>());
}