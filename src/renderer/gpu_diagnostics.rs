use std::time::{Duration, Instant};
use wgpu::{Adapter, Device, Instance};
use anyhow::Result;

/// GPU diagnostics and validation utilities
pub struct GpuDiagnostics;

impl GpuDiagnostics {
    /// Run comprehensive GPU diagnostics
    pub async fn run_diagnostics(instance: &Instance) -> DiagnosticsReport {
        let mut report = DiagnosticsReport::default();
        let start = Instant::now();
        
        // Check available backends
        report.available_backends = Self::check_backends();
        
        // Enumerate all adapters
        let adapters = instance.enumerate_adapters(wgpu::Backends::all());
        for adapter in adapters {
            let info = adapter.get_info();
            report.available_adapters.push(AdapterInfo {
                name: info.name,
                backend: format!("{:?}", info.backend),
                device_type: format!("{:?}", info.device_type),
                vendor_id: info.vendor,
                device_id: info.device,
                driver_info: info.driver_info,
                features: Self::get_adapter_features(&adapter),
                limits: Self::get_adapter_limits(&adapter),
            });
        }
        
        report.diagnostics_time = start.elapsed();
        report
    }
    
    /// Check which backends are available
    fn check_backends() -> Vec<String> {
        let mut backends = Vec::new();
        
        if cfg!(target_os = "windows") {
            backends.push("DirectX 12".to_string());
            backends.push("DirectX 11".to_string());
            backends.push("Vulkan".to_string());
        } else if cfg!(target_os = "linux") {
            backends.push("Vulkan".to_string());
            backends.push("OpenGL".to_string());
        } else if cfg!(target_os = "macos") {
            backends.push("Metal".to_string());
        } else if cfg!(target_arch = "wasm32") {
            backends.push("WebGPU".to_string());
            backends.push("WebGL2".to_string());
        }
        
        backends
    }
    
    /// Get important adapter features
    fn get_adapter_features(adapter: &Adapter) -> Vec<String> {
        let features = adapter.features();
        let mut feature_list = Vec::new();
        
        // Check important features
        if features.contains(wgpu::Features::DEPTH_CLIP_CONTROL) {
            feature_list.push("Depth Clip Control".to_string());
        }
        if features.contains(wgpu::Features::TEXTURE_COMPRESSION_BC) {
            feature_list.push("BC Texture Compression".to_string());
        }
        if features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            feature_list.push("Timestamp Queries".to_string());
        }
        if features.contains(wgpu::Features::INDIRECT_FIRST_INSTANCE) {
            feature_list.push("Indirect First Instance".to_string());
        }
        
        feature_list
    }
    
    /// Get important adapter limits
    fn get_adapter_limits(adapter: &Adapter) -> AdapterLimits {
        let limits = adapter.limits();
        AdapterLimits {
            max_texture_dimension_2d: limits.max_texture_dimension_2d,
            max_texture_dimension_3d: limits.max_texture_dimension_3d,
            max_buffer_size: limits.max_buffer_size,
            max_vertex_buffers: limits.max_vertex_buffers,
            max_bind_groups: limits.max_bind_groups,
            max_compute_workgroup_size_x: limits.max_compute_workgroup_size_x,
            max_compute_workgroup_size_y: limits.max_compute_workgroup_size_y,
            max_compute_workgroup_size_z: limits.max_compute_workgroup_size_z,
        }
    }
    
    /// Validate GPU capabilities for the engine
    pub fn validate_capabilities(adapter: &Adapter) -> ValidationResult {
        let mut result = ValidationResult {
            is_valid: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        
        let limits = adapter.limits();
        let info = adapter.get_info();
        
        // Check minimum texture size
        if limits.max_texture_dimension_2d < 4096 {
            result.warnings.push(format!(
                "Low max texture dimension: {} (recommended: 4096+ for high-quality terrain textures)",
                limits.max_texture_dimension_2d
            ));
        } else if limits.max_texture_dimension_2d >= 8192 {
            log::info!("[GPU Validation] Excellent texture support: {} (can use high-res terrain textures)", 
                     limits.max_texture_dimension_2d);
        }
        
        // Check buffer size
        if limits.max_buffer_size < 256 * 1024 * 1024 {
            result.warnings.push(format!(
                "Low max buffer size: {} MB (recommended: 256+ MB)",
                limits.max_buffer_size / 1024 / 1024
            ));
        }
        
        // Check for software renderer
        if info.device_type == wgpu::DeviceType::Cpu {
            result.warnings.push("Using CPU/software renderer - performance will be poor".to_string());
        }
        
        // Check for known problematic configurations
        if cfg!(target_os = "linux") && info.backend == wgpu::Backend::Gl {
            result.warnings.push("OpenGL backend on Linux may have compatibility issues".to_string());
        }
        
        result
    }
    
    /// Test basic GPU operations
    pub async fn test_gpu_operations(device: &Device) -> OperationTestResult {
        let mut result = OperationTestResult::default();
        let start = Instant::now();
        
        // Test buffer creation
        let buffer_start = Instant::now();
        match Self::test_buffer_creation(device) {
            Ok(size) => {
                result.buffer_test = TestStatus::Success(buffer_start.elapsed());
                result.max_tested_buffer_size = size;
            }
            Err(e) => {
                result.buffer_test = TestStatus::Failed(e.to_string());
            }
        }
        
        // Test texture creation
        let texture_start = Instant::now();
        match Self::test_texture_creation(device) {
            Ok(dim) => {
                result.texture_test = TestStatus::Success(texture_start.elapsed());
                result.max_tested_texture_dimension = dim;
            }
            Err(e) => {
                result.texture_test = TestStatus::Failed(e.to_string());
            }
        }
        
        // Test shader compilation
        let shader_start = Instant::now();
        match Self::test_shader_compilation(device) {
            Ok(_) => {
                result.shader_test = TestStatus::Success(shader_start.elapsed());
            }
            Err(e) => {
                result.shader_test = TestStatus::Failed(e.to_string());
            }
        }
        
        result.total_test_time = start.elapsed();
        result
    }
    
    fn test_buffer_creation(device: &Device) -> Result<u64> {
        // Try creating progressively larger buffers
        let sizes = [
            1024 * 1024,           // 1 MB
            16 * 1024 * 1024,      // 16 MB
            64 * 1024 * 1024,      // 64 MB
            256 * 1024 * 1024,     // 256 MB
        ];
        
        let mut max_size = 0u64;
        for &size in &sizes {
            match device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Test Buffer"),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }) {
                buffer => {
                    max_size = size;
                    drop(buffer); // Immediately release
                }
            }
        }
        
        Ok(max_size)
    }
    
    fn test_texture_creation(device: &Device) -> Result<u32> {
        // Get device limits to avoid testing beyond hardware capabilities
        let device_limits = device.limits();
        let max_hardware_dimension = device_limits.max_texture_dimension_2d;
        
        log::debug!("[GPU Test] Hardware max texture dimension: {}", max_hardware_dimension);
        
        // Try creating progressively larger textures, but stop at hardware limit
        let dimensions = [512, 1024, 2048, 4096, 8192, 16384];
        
        let mut max_dim = 0u32;
        for &dim in &dimensions {
            // Skip dimensions that exceed hardware limits
            if dim > max_hardware_dimension {
                log::debug!("[GPU Test] Skipping {}x{} texture - exceeds hardware limit of {}", 
                          dim, dim, max_hardware_dimension);
                break;
            }
            
            // Try to create the texture, catching any errors
            log::debug!("[GPU Test] Attempting to create {}x{} texture", dim, dim);
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Test Texture"),
                    size: wgpu::Extent3d {
                        width: dim,
                        height: dim,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                })
            })) {
                Ok(texture) => {
                    max_dim = dim;
                    drop(texture); // Immediately release
                    log::debug!("[GPU Test] Successfully created {}x{} texture", dim, dim);
                }
                Err(_) => {
                    log::debug!("[GPU Test] Failed to create {}x{} texture - unexpected error", dim, dim);
                    break; // Stop trying larger sizes
                }
            }
        }
        
        if max_dim == 0 {
            log::warn!("[GPU Test] Could not create any test textures!");
        } else {
            log::info!("[GPU Test] Maximum tested texture dimension: {}x{}", max_dim, max_dim);
        }
        
        Ok(max_dim)
    }
    
    fn test_shader_compilation(device: &Device) -> Result<()> {
        // Test compiling a simple shader
        let shader_source = r#"
            @vertex
            fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
                var pos = array<vec2<f32>, 3>(
                    vec2<f32>(-1.0, -1.0),
                    vec2<f32>( 3.0, -1.0),
                    vec2<f32>(-1.0,  3.0)
                );
                return vec4<f32>(pos[in_vertex_index], 0.0, 1.0);
            }
            
            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return vec4<f32>(1.0, 0.0, 0.0, 1.0);
            }
        "#;
        
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Test Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });
        
        Ok(())
    }
}

/// Diagnostics report structure
#[derive(Debug, Default)]
pub struct DiagnosticsReport {
    pub available_backends: Vec<String>,
    pub available_adapters: Vec<AdapterInfo>,
    pub diagnostics_time: Duration,
}

/// Adapter information
#[derive(Debug)]
pub struct AdapterInfo {
    pub name: String,
    pub backend: String,
    pub device_type: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub driver_info: String,
    pub features: Vec<String>,
    pub limits: AdapterLimits,
}

/// Important adapter limits
#[derive(Debug)]
pub struct AdapterLimits {
    pub max_texture_dimension_2d: u32,
    pub max_texture_dimension_3d: u32,
    pub max_buffer_size: u64,
    pub max_vertex_buffers: u32,
    pub max_bind_groups: u32,
    pub max_compute_workgroup_size_x: u32,
    pub max_compute_workgroup_size_y: u32,
    pub max_compute_workgroup_size_z: u32,
}

/// Validation result
#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// GPU operation test result
#[derive(Debug, Default)]
pub struct OperationTestResult {
    pub buffer_test: TestStatus,
    pub texture_test: TestStatus,
    pub shader_test: TestStatus,
    pub max_tested_buffer_size: u64,
    pub max_tested_texture_dimension: u32,
    pub total_test_time: Duration,
}

/// Test status
#[derive(Debug)]
pub enum TestStatus {
    NotRun,
    Success(Duration),
    Failed(String),
}

impl Default for TestStatus {
    fn default() -> Self {
        TestStatus::NotRun
    }
}

impl DiagnosticsReport {
    /// Print a formatted report
    pub fn print_report(&self) {
        log::info!("=== GPU Diagnostics Report ===");
        log::info!("Diagnostics completed in: {:?}", self.diagnostics_time);
        
        log::info!("\nAvailable Backends:");
        for backend in &self.available_backends {
            log::info!("  - {}", backend);
        }
        
        log::info!("\nAvailable Adapters:");
        for (i, adapter) in self.available_adapters.iter().enumerate() {
            log::info!("\n  Adapter {}:", i);
            log::info!("    Name: {}", adapter.name);
            log::info!("    Backend: {}", adapter.backend);
            log::info!("    Type: {}", adapter.device_type);
            log::info!("    Vendor ID: 0x{:04x}", adapter.vendor_id);
            log::info!("    Device ID: 0x{:04x}", adapter.device_id);
            log::info!("    Driver: {}", adapter.driver_info);
            
            log::info!("    Features:");
            for feature in &adapter.features {
                log::info!("      - {}", feature);
            }
            
            log::info!("    Limits:");
            log::info!("      Max Texture 2D: {}", adapter.limits.max_texture_dimension_2d);
            log::info!("      Max Buffer Size: {} MB", adapter.limits.max_buffer_size / 1024 / 1024);
            log::info!("      Max Vertex Buffers: {}", adapter.limits.max_vertex_buffers);
        }
    }
}

impl ValidationResult {
    /// Print validation results
    pub fn print_results(&self) {
        if self.is_valid {
            log::info!("[GPU Validation] ✓ GPU capabilities validated successfully");
        } else {
            log::error!("[GPU Validation] ✗ GPU validation failed");
        }
        
        if !self.warnings.is_empty() {
            log::warn!("[GPU Validation] Warnings:");
            for warning in &self.warnings {
                log::warn!("  - {}", warning);
            }
        }
        
        if !self.errors.is_empty() {
            log::error!("[GPU Validation] Errors:");
            for error in &self.errors {
                log::error!("  - {}", error);
            }
        }
    }
}

impl OperationTestResult {
    /// Print test results
    pub fn print_results(&self) {
        log::info!("=== GPU Operation Tests ===");
        log::info!("Total test time: {:?}", self.total_test_time);
        
        match &self.buffer_test {
            TestStatus::Success(duration) => {
                log::info!("✓ Buffer creation: Success ({:?})", duration);
                log::info!("  Max tested size: {} MB", self.max_tested_buffer_size / 1024 / 1024);
            }
            TestStatus::Failed(error) => {
                log::error!("✗ Buffer creation: Failed - {}", error);
            }
            TestStatus::NotRun => {
                log::warn!("- Buffer creation: Not tested");
            }
        }
        
        match &self.texture_test {
            TestStatus::Success(duration) => {
                log::info!("✓ Texture creation: Success ({:?})", duration);
                log::info!("  Max tested dimension: {}x{}", self.max_tested_texture_dimension, self.max_tested_texture_dimension);
            }
            TestStatus::Failed(error) => {
                log::error!("✗ Texture creation: Failed - {}", error);
            }
            TestStatus::NotRun => {
                log::warn!("- Texture creation: Not tested");
            }
        }
        
        match &self.shader_test {
            TestStatus::Success(duration) => {
                log::info!("✓ Shader compilation: Success ({:?})", duration);
            }
            TestStatus::Failed(error) => {
                log::error!("✗ Shader compilation: Failed - {}", error);
            }
            TestStatus::NotRun => {
                log::warn!("- Shader compilation: Not tested");
            }
        }
    }
}