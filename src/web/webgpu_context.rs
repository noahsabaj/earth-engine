use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;
use wgpu::{Adapter, Device, Queue, Surface, SurfaceConfiguration};
use crate::web::WebError;

#[cfg(target_arch = "wasm32")]
use instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

/// Configuration for WebGPU initialization
#[derive(Debug, Clone)]
pub struct WebGpuConfig {
    /// Power preference for adapter selection
    pub power_preference: wgpu::PowerPreference,
    /// Required features
    pub required_features: wgpu::Features,
    /// Required limits
    pub required_limits: wgpu::Limits,
    /// Preferred texture format
    pub texture_format: wgpu::TextureFormat,
}

impl Default for WebGpuConfig {
    fn default() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::HighPerformance,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            texture_format: wgpu::TextureFormat::Bgra8UnormSrgb,
        }
    }
}

/// WebGPU context for browser environment
pub struct WebGpuContext {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub surface_config: SurfaceConfiguration,
    pub adapter: Adapter,
    canvas: HtmlCanvasElement,
}

impl WebGpuContext {
    /// Create a new WebGPU context for the given canvas
    pub async fn new(canvas: &HtmlCanvasElement) -> Result<Self, WebError> {
        Self::with_config(canvas, WebGpuConfig::default()).await
    }
    
    /// Create a new WebGPU context with custom configuration
    pub async fn with_config(
        canvas: &HtmlCanvasElement,
        config: WebGpuConfig,
    ) -> Result<Self, WebError> {
        log::info!("[WebGpuContext] Initializing WebGPU context");
        let init_start = Instant::now();
        
        // Check WebGPU support first
        let window = web_sys::window().ok_or(WebError::JsError("No window object".to_string()))?;
        let navigator = window.navigator();
        
        // Log GPU capabilities
        if let Ok(gpu) = js_sys::Reflect::get(&navigator, &"gpu".into()) {
            if !gpu.is_undefined() {
                log::info!("[WebGpuContext] WebGPU API is available");
            } else {
                log::warn!("[WebGpuContext] WebGPU API not available, will fall back to WebGL");
            }
        }
        
        // Create WGPU instance with WebGL fallback
        log::info!("[WebGpuContext] Creating WGPU instance...");
        let instance_start = Instant::now();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            dx12_shader_compiler: Default::default(),
        });
        let instance_time = instance_start.elapsed();
        log::info!("[WebGpuContext] WGPU instance created in {:?}", instance_time);
        
        // Create surface from canvas with error diagnostics
        log::info!("[WebGpuContext] Creating surface from canvas...");
        let surface_start = Instant::now();
        
        // SAFETY: Creating a surface from a canvas is platform-specific but safe
        // - The canvas is a valid HTMLCanvasElement from the DOM
        // - The canvas will outlive the surface due to Arc reference counting
        // - wgpu handles the platform-specific WebGPU/WebGL context creation
        // - No raw pointers or memory unsafety involved
        let surface = unsafe {
            match instance.create_surface_from_canvas(canvas.clone()) {
                Ok(surf) => {
                    let surface_time = surface_start.elapsed();
                    log::info!("[WebGpuContext] Surface created successfully in {:?}", surface_time);
                    surf
                }
                Err(e) => {
                    log::error!("[WebGpuContext] Failed to create surface: {:?}", e);
                    log::error!("[WebGpuContext] This may be due to:");
                    log::error!("[WebGpuContext] - Canvas not attached to DOM");
                    log::error!("[WebGpuContext] - Browser WebGPU/WebGL support issues");
                    log::error!("[WebGpuContext] - Canvas context already in use");
                    return Err(WebError::JsError(format!("Surface creation failed: {:?}", e)));
                }
            }
        };
        
        // Request adapter with timeout and fallback
        log::info!("[WebGpuContext] Requesting GPU adapter...");
        let adapter_start = Instant::now();
        
        // Try with requested power preference first
        let mut adapter_options = wgpu::RequestAdapterOptions {
            power_preference: config.power_preference,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        };
        
        let adapter = match instance.request_adapter(&adapter_options).await {
            Some(adapter) => {
                let adapter_time = adapter_start.elapsed();
                let info = adapter.get_info();
                log::info!("[WebGpuContext] Adapter found in {:?}", adapter_time);
                log::info!("[WebGpuContext] Adapter: {} ({:?})", info.name, info.device_type);
                log::info!("[WebGpuContext] Backend: {:?}", info.backend);
                adapter
            }
            None => {
                log::warn!("[WebGpuContext] No adapter found with preference {:?}, trying fallback", config.power_preference);
                
                // Try fallback adapter
                adapter_options.force_fallback_adapter = true;
                match instance.request_adapter(&adapter_options).await {
                    Some(adapter) => {
                        let info = adapter.get_info();
                        log::warn!("[WebGpuContext] Using fallback adapter: {}", info.name);
                        adapter
                    }
                    None => {
                        log::error!("[WebGpuContext] No GPU adapter found!");
                        log::error!("[WebGpuContext] This browser may not support WebGPU/WebGL");
                        return Err(WebError::AdapterError);
                    }
                }
            }
        };
        
        // Request device and queue with error handling
        log::info!("[WebGpuContext] Requesting device...");
        let device_start = Instant::now();
        
        let (device, queue) = match adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("WebGPU Device"),
                    features: config.required_features,
                    limits: config.required_limits.clone(),
                },
                None,
            )
            .await
        {
            Ok((dev, q)) => {
                let device_time = device_start.elapsed();
                log::info!("[WebGpuContext] Device created successfully in {:?}", device_time);
                
                // Set up device error handler
                dev.on_uncaptured_error(Box::new(|error| {
                    web_sys::console::error_1(&format!("[WebGPU] Uncaptured device error: {:?}", error).into());
                }));
                
                (dev, q)
            }
            Err(e) => {
                log::error!("[WebGpuContext] Failed to create device: {:?}", e);
                log::error!("[WebGpuContext] Requested features: {:?}", config.required_features);
                log::error!("[WebGpuContext] Requested limits: {:?}", config.required_limits);
                return Err(WebError::DeviceError);
            }
        };
        
        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        
        let width = canvas.client_width() as u32;
        let height = canvas.client_height() as u32;
        
        let surface_config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        
        surface.configure(&device, &surface_config);
        
        let total_time = init_start.elapsed();
        log::info!("[WebGpuContext] WebGPU context initialized successfully in {:?}", total_time);
        log::info!("[WebGpuContext] Using backend: {:?}", adapter.get_info().backend);
        
        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            adapter,
            canvas: canvas.clone(),
        })
    }
    
    /// Resize the surface
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }
    
    /// Get current canvas size
    pub fn canvas_size(&self) -> (u32, u32) {
        (
            self.canvas.client_width() as u32,
            self.canvas.client_height() as u32,
        )
    }
    
    /// Check if size has changed and resize if necessary
    pub fn update_size(&mut self) {
        let (width, height) = self.canvas_size();
        if width != self.surface_config.width || height != self.surface_config.height {
            self.resize(width, height);
        }
    }
    
    /// Get the current surface texture for rendering
    pub fn get_current_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }
    
    /// Get device limits
    pub fn limits(&self) -> wgpu::Limits {
        self.device.limits()
    }
    
    /// Get adapter info
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }
    
    /// Log performance info
    pub fn log_performance_info(&self) {
        let info = self.adapter_info();
        log::info!("=== WebGPU Performance Info ===");
        log::info!("Adapter: {}", info.name);
        log::info!("Backend: {:?}", info.backend);
        log::info!("Device Type: {:?}", info.device_type);
        log::info!("Vendor: {}", info.vendor);
        
        let limits = self.limits();
        log::info!("Max buffer size: {} MB", limits.max_buffer_size / 1024 / 1024);
        log::info!("Max texture dimension 2D: {}", limits.max_texture_dimension_2d);
        log::info!("Max workgroup size X: {}", limits.max_compute_workgroup_size_x);
        log::info!("Max workgroups per dimension: {}", limits.max_compute_workgroups_per_dimension);
    }
}