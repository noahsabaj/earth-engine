#[repr(C)]
#[derive(Copy, Clone)]
struct CameraUniform {
    view: [[f32; 4]; 4],        // 64 bytes
    projection: [[f32; 4]; 4],   // 64 bytes
    view_proj: [[f32; 4]; 4],    // 64 bytes
    position: [f32; 3],          // 12 bytes
    _padding: f32,               // 4 bytes
}

fn main() {
    println\!("Size of CameraUniform: {} bytes", std::mem::size_of::<CameraUniform>());
    println\!("Expected: 208 bytes");
    println\!("Breakdown:");
    println\!("  view: {} bytes", std::mem::size_of::<[[f32; 4]; 4]>());
    println\!("  projection: {} bytes", std::mem::size_of::<[[f32; 4]; 4]>());
    println\!("  view_proj: {} bytes", std::mem::size_of::<[[f32; 4]; 4]>());
    println\!("  position: {} bytes", std::mem::size_of::<[f32; 3]>());
    println\!("  _padding: {} bytes", std::mem::size_of::<f32>());
    println\!("  Total: 64 + 64 + 64 + 12 + 4 = {}", 64 + 64 + 64 + 12 + 4);
}
