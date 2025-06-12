fn main() {
    let default_limits = wgpu::Limits::default();
    println!("Default max_texture_dimension_2d: {}", default_limits.max_texture_dimension_2d);
    
    let downlevel_limits = wgpu::Limits::downlevel_defaults();
    println!("Downlevel max_texture_dimension_2d: {}", downlevel_limits.max_texture_dimension_2d);
    
    let webgl2_limits = wgpu::Limits::downlevel_webgl2_defaults();
    println!("WebGL2 max_texture_dimension_2d: {}", webgl2_limits.max_texture_dimension_2d);
}