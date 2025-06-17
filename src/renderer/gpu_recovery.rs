use wgpu::{Adapter, Device, Instance, Surface};
use std::time::Duration;

/// GPU recovery strategies for initialization failures
pub struct GpuRecovery;

impl GpuRecovery {
    /// Try to recover from adapter request failure
    pub async fn recover_adapter_request(
        instance: &Instance,
        surface: &Surface<'static>,
    ) -> Option<Adapter> {
        log::warn!("[GPU Recovery] Attempting adapter recovery strategies...");
        
        // Strategy 1: Try different power preferences
        let power_preferences = [
            wgpu::PowerPreference::LowPower,
            wgpu::PowerPreference::HighPerformance,
            wgpu::PowerPreference::None,
        ];
        
        for (i, power_pref) in power_preferences.iter().enumerate() {
            log::info!("[GPU Recovery] Strategy {}: Trying {:?} power preference", i + 1, power_pref);
            
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: *power_pref,
                    compatible_surface: Some(surface),
                    force_fallback_adapter: false,
                })
                .await;
                
            if let Some(adapter) = adapter {
                log::info!("[GPU Recovery] Success with {:?} preference", power_pref);
                return Some(adapter);
            }
        }
        
        // Strategy 2: Try without surface compatibility
        log::info!("[GPU Recovery] Strategy 4: Trying without surface compatibility");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await;
            
        if adapter.is_some() {
            log::warn!("[GPU Recovery] Found adapter without surface compatibility - may have issues");
            return adapter;
        }
        
        // Strategy 3: Force fallback adapter
        log::info!("[GPU Recovery] Strategy 5: Forcing fallback adapter");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                compatible_surface: Some(surface),
                force_fallback_adapter: true,
            })
            .await;
            
        if adapter.is_some() {
            log::warn!("[GPU Recovery] Using fallback adapter - performance will be limited");
            return adapter;
        }
        
        log::error!("[GPU Recovery] All adapter recovery strategies failed");
        None
    }
    
    /// Try to recover from device creation failure
    pub async fn recover_device_creation(
        adapter: &Adapter,
    ) -> Option<(Device, wgpu::Queue)> {
        log::warn!("[GPU Recovery] Attempting device creation recovery...");
        
        // Strategy 1: Try with minimal limits
        log::info!("[GPU Recovery] Strategy 1: Using minimal limits");
        let minimal_limits = wgpu::Limits::downlevel_defaults();
        
        match adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Hearth Engine Device (Recovery)"),
                    required_features: wgpu::Features::empty(),
                    required_limits: minimal_limits,
                },
                None,
            )
            .await
        {
            Ok(device_queue) => {
                log::info!("[GPU Recovery] Device created with minimal limits");
                return Some(device_queue);
            }
            Err(e) => {
                log::error!("[GPU Recovery] Failed with minimal limits: {}", e);
            }
        }
        
        // Strategy 2: Try with downlevel limits
        log::info!("[GPU Recovery] Strategy 2: Using downlevel WebGL2 limits");
        match adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Hearth Engine Device (WebGL2)"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                },
                None,
            )
            .await
        {
            Ok(device_queue) => {
                log::info!("[GPU Recovery] Device created with WebGL2 limits");
                return Some(device_queue);
            }
            Err(e) => {
                log::error!("[GPU Recovery] Failed with WebGL2 limits: {}", e);
            }
        }
        
        log::error!("[GPU Recovery] All device creation recovery strategies failed");
        None
    }
    
    /// Check if we can use a fallback rendering mode
    pub fn can_use_fallback_rendering(adapter: &Adapter) -> bool {
        let info = adapter.get_info();
        
        // Check if we're on a software renderer
        if info.device_type == wgpu::DeviceType::Cpu {
            log::warn!("[GPU Recovery] CPU/Software renderer detected - fallback rendering recommended");
            return true;
        }
        
        // Check for known problematic configurations
        if cfg!(target_os = "linux") && info.backend == wgpu::Backend::Gl {
            log::warn!("[GPU Recovery] OpenGL on Linux detected - may need fallback rendering");
            return true;
        }
        
        false
    }
    
    /// Get recommended settings for problematic GPUs
    pub fn get_fallback_settings(adapter: &Adapter) -> FallbackSettings {
        let info = adapter.get_info();
        let limits = adapter.limits();
        
        let mut settings = FallbackSettings::default();
        
        // Adjust based on device type
        match info.device_type {
            wgpu::DeviceType::Cpu => {
                settings.max_chunks_per_frame = 1;
                settings.chunk_size = 16;
                settings.render_distance = 2;
                settings.enable_shadows = false;
                settings.enable_reflections = false;
                settings.texture_quality = TextureQuality::Low;
            }
            wgpu::DeviceType::IntegratedGpu => {
                settings.max_chunks_per_frame = 4;
                settings.chunk_size = 24;
                settings.render_distance = 4;
                settings.enable_shadows = false;
                settings.texture_quality = TextureQuality::Medium;
            }
            _ => {}
        }
        
        // Adjust based on memory limits
        if limits.max_buffer_size < 256 * 1024 * 1024 {
            settings.chunk_size = settings.chunk_size.min(16);
            settings.render_distance = settings.render_distance.min(4);
        }
        
        // Adjust based on texture limits
        if limits.max_texture_dimension_2d < 4096 {
            settings.texture_quality = TextureQuality::Low;
        }
        
        log::info!("[GPU Recovery] Recommended fallback settings: {:?}", settings);
        settings
    }
}

/// Fallback rendering settings for problematic GPUs
#[derive(Debug, Clone)]
pub struct FallbackSettings {
    pub chunk_size: u32,
    pub render_distance: u32,
    pub max_chunks_per_frame: usize,
    pub enable_shadows: bool,
    pub enable_reflections: bool,
    pub texture_quality: TextureQuality,
}

impl Default for FallbackSettings {
    fn default() -> Self {
        Self {
            chunk_size: 32,
            render_distance: 8,
            max_chunks_per_frame: 8,
            enable_shadows: true,
            enable_reflections: true,
            texture_quality: TextureQuality::High,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TextureQuality {
    Low,
    Medium,
    High,
}

/// GPU health monitor
pub struct GpuHealthMonitor {
    error_count: usize,
    last_error_time: Option<std::time::Instant>,
    recovery_attempts: usize,
}

impl GpuHealthMonitor {
    pub fn new() -> Self {
        Self {
            error_count: 0,
            last_error_time: None,
            recovery_attempts: 0,
        }
    }
    
    pub fn record_error(&mut self) {
        self.error_count += 1;
        self.last_error_time = Some(std::time::Instant::now());
        
        log::warn!("[GPU Health] Error recorded. Total errors: {}", self.error_count);
    }
    
    pub fn record_recovery_attempt(&mut self) {
        self.recovery_attempts += 1;
        log::info!("[GPU Health] Recovery attempt #{}", self.recovery_attempts);
    }
    
    pub fn should_attempt_recovery(&self) -> bool {
        // Don't attempt too many recoveries
        if self.recovery_attempts >= 3 {
            log::error!("[GPU Health] Maximum recovery attempts reached");
            return false;
        }
        
        // Check if errors are happening too frequently
        if let Some(last_error) = self.last_error_time {
            if last_error.elapsed() < Duration::from_secs(5) && self.error_count > 5 {
                log::error!("[GPU Health] Too many errors in short time - stopping recovery");
                return false;
            }
        }
        
        true
    }
    
    pub fn reset(&mut self) {
        self.error_count = 0;
        self.last_error_time = None;
        self.recovery_attempts = 0;
        log::info!("[GPU Health] Monitor reset");
    }
}