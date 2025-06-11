use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;
use wgpu::{Adapter, Device, Queue, Surface, SurfaceConfiguration};
use crate::web::WebError;

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
        log::info!("Initializing WebGPU context");
        
        // WebGPU support will be checked by wgpu instance
        
        // Create WGPU instance with WebGL fallback
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            dx12_shader_compiler: Default::default(),
        });
        
        // Create surface from canvas
        let surface = unsafe {
            instance.create_surface_from_canvas(canvas.clone())
                .map_err(|e| WebError::JsError(format!("Failed to create surface: {:?}", e)))?
        };
        
        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: config.power_preference,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or(WebError::AdapterError)?;
        
        log::info!("Got adapter: {:?}", adapter.get_info());
        
        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("WebGPU Device"),
                    features: config.required_features,
                    limits: config.required_limits.clone(),
                },
                None,
            )
            .await
            .map_err(|e| WebError::DeviceError)?;
        
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
        
        log::info!("WebGPU context initialized successfully");
        
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