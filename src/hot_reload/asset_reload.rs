use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use wgpu::{Device, Queue, Texture, TextureView, TextureDescriptor, TextureFormat};
use image::{DynamicImage, ImageFormat};
use super::{WatchEvent, WatchEventType, HotReloadResult, HotReloadErrorContext, asset_reload_error};
use crate::error::EngineError;

/// Asset type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssetType {
    Texture,
    Model,
    Sound,
    Config,
    Script,
    Unknown,
}

impl AssetType {
    /// Detect asset type from file extension
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|e| e.to_str()) {
            Some("png") | Some("jpg") | Some("jpeg") | Some("bmp") | Some("tga") => AssetType::Texture,
            Some("obj") | Some("gltf") | Some("glb") | Some("fbx") => AssetType::Model,
            Some("wav") | Some("mp3") | Some("ogg") | Some("flac") => AssetType::Sound,
            Some("toml") | Some("json") | Some("yaml") | Some("ron") => AssetType::Config,
            Some("lua") | Some("rhai") | Some("wren") => AssetType::Script,
            _ => AssetType::Unknown,
        }
    }
}

/// Asset identifier
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct AssetId {
    /// Asset name/key
    pub name: String,
    
    /// Asset type
    pub asset_type: AssetType,
}

/// Loaded texture asset
pub struct TextureAsset {
    /// GPU texture
    pub texture: Arc<Texture>,
    
    /// Texture view
    pub view: Arc<TextureView>,
    
    /// Original image data
    pub image: DynamicImage,
    
    /// Texture format
    pub format: TextureFormat,
}

/// Generic asset data
pub enum AssetData {
    Texture(TextureAsset),
    Model(Vec<u8>), // Raw model data
    Sound(Vec<u8>), // Raw audio data
    Config(String), // Config file content
    Script(String), // Script source
}

/// Cached asset
pub struct CachedAsset {
    /// Asset data
    pub data: AssetData,
    
    /// File path
    pub path: PathBuf,
    
    /// Last modified time
    pub last_modified: std::time::SystemTime,
    
    /// Reload callbacks
    pub callbacks: Vec<String>,
}

/// Asset reloader
pub struct AssetReloader {
    /// Device reference
    device: Arc<Device>,
    
    /// Queue reference
    queue: Arc<Queue>,
    
    /// Asset cache
    cache: Arc<RwLock<HashMap<AssetId, CachedAsset>>>,
    
    /// Path to asset ID mapping
    path_map: Arc<RwLock<HashMap<PathBuf, AssetId>>>,
    
    /// Reload callbacks
    callbacks: Arc<RwLock<HashMap<String, Box<dyn Fn(&AssetData) + Send + Sync>>>>,
    
    /// Asset directories
    asset_dirs: Vec<PathBuf>,
}

impl AssetReloader {
    /// Create new asset reloader
    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {
        Self {
            device,
            queue,
            cache: Arc::new(RwLock::new(HashMap::new())),
            path_map: Arc::new(RwLock::new(HashMap::new())),
            callbacks: Arc::new(RwLock::new(HashMap::new())),
            asset_dirs: vec![PathBuf::from("assets")],
        }
    }
    
    /// Add asset directory
    pub fn add_asset_dir(&mut self, path: impl AsRef<Path>) {
        self.asset_dirs.push(path.as_ref().to_path_buf());
    }
    
    /// Load texture asset
    pub fn load_texture(
        &self,
        name: &str,
        path: impl AsRef<Path>,
    ) -> HotReloadResult<Arc<Texture>> {
        let path = path.as_ref();
        
        // Load image
        let mut image = image::open(path)
            .map_err(|e| asset_reload_error(name, format!("Failed to load image: {}", e)))?;
        
        // Get GPU texture size limits
        let device_limits = self.device.limits();
        let max_texture_dimension = device_limits.max_texture_dimension_2d;
        
        // Validate and potentially resize image dimensions
        let original_dimensions = (image.width(), image.height());
        let (validated_width, validated_height) = validate_and_resize_image(
            &mut image,
            max_texture_dimension,
            name,
        );
        
        // Log if image was resized
        if (validated_width, validated_height) != original_dimensions {
            log::warn!(
                "[AssetReloader::load_texture] Image '{}' resized from {}x{} to {}x{} due to GPU texture limit ({})",
                name,
                original_dimensions.0,
                original_dimensions.1,
                validated_width,
                validated_height,
                max_texture_dimension
            );
        }
        
        // Convert to RGBA8
        let rgba = image.to_rgba8();
        let dimensions = rgba.dimensions();
        
        // Create texture with validated dimensions
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        
        let texture = self.device.create_texture(&TextureDescriptor {
            label: Some(name),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        // Upload data
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );
        
        let texture = Arc::new(texture);
        let view = Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        
        // Cache asset
        let asset_id = AssetId {
            name: name.to_string(),
            asset_type: AssetType::Texture,
        };
        
        let cached = CachedAsset {
            data: AssetData::Texture(TextureAsset {
                texture: texture.clone(),
                view,
                image,
                format: TextureFormat::Rgba8UnormSrgb,
            }),
            path: path.to_path_buf(),
            last_modified: std::fs::metadata(path)
                .map(|m| m.modified().unwrap_or(std::time::SystemTime::now()))
                .unwrap_or(std::time::SystemTime::now()),
            callbacks: Vec::new(),
        };
        
        self.cache.write()
            .hot_reload_context("asset_cache")?
            .insert(asset_id.clone(), cached);
        self.path_map.write()
            .hot_reload_context("path_map")?
            .insert(path.to_path_buf(), asset_id);
        
        Ok(texture)
    }
    
    /// Load generic asset
    pub fn load_asset(
        &self,
        name: &str,
        path: impl AsRef<Path>,
    ) -> HotReloadResult<()> {
        let path = path.as_ref();
        let asset_type = AssetType::from_path(path);
        
        match asset_type {
            AssetType::Texture => {
                self.load_texture(name, path)?;
            }
            AssetType::Config => {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| EngineError::IoError {
                        path: path.to_string_lossy().to_string(),
                        error: e.to_string(),
                    })?;
                
                let asset_id = AssetId {
                    name: name.to_string(),
                    asset_type: AssetType::Config,
                };
                
                let cached = CachedAsset {
                    data: AssetData::Config(content),
                    path: path.to_path_buf(),
                    last_modified: std::fs::metadata(path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .unwrap_or(std::time::SystemTime::now()),
                    callbacks: Vec::new(),
                };
                
                self.cache.write()
                    .hot_reload_context("asset_cache")?
                    .insert(asset_id.clone(), cached);
                self.path_map.write()
                    .hot_reload_context("path_map")?
                    .insert(path.to_path_buf(), asset_id);
            }
            AssetType::Script => {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| EngineError::IoError {
                        path: path.to_string_lossy().to_string(),
                        error: e.to_string(),
                    })?;
                
                let asset_id = AssetId {
                    name: name.to_string(),
                    asset_type: AssetType::Script,
                };
                
                let cached = CachedAsset {
                    data: AssetData::Script(content),
                    path: path.to_path_buf(),
                    last_modified: std::fs::metadata(path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .unwrap_or(std::time::SystemTime::now()),
                    callbacks: Vec::new(),
                };
                
                self.cache.write()
                    .hot_reload_context("asset_cache")?
                    .insert(asset_id.clone(), cached);
                self.path_map.write()
                    .hot_reload_context("path_map")?
                    .insert(path.to_path_buf(), asset_id);
            }
            _ => {
                // Load as binary data
                let data = std::fs::read(path)
                    .map_err(|e| EngineError::IoError {
                        path: path.to_string_lossy().to_string(),
                        error: e.to_string(),
                    })?;
                
                let asset_id = AssetId {
                    name: name.to_string(),
                    asset_type: asset_type.clone(),
                };
                
                let asset_data = match asset_type {
                    AssetType::Model => AssetData::Model(data),
                    AssetType::Sound => AssetData::Sound(data),
                    _ => return Err(asset_reload_error(name, "Unsupported asset type")),
                };
                
                let cached = CachedAsset {
                    data: asset_data,
                    path: path.to_path_buf(),
                    last_modified: std::fs::metadata(path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .unwrap_or(std::time::SystemTime::now()),
                    callbacks: Vec::new(),
                };
                
                self.cache.write()
                    .hot_reload_context("asset_cache")?
                    .insert(asset_id.clone(), cached);
                self.path_map.write()
                    .hot_reload_context("path_map")?
                    .insert(path.to_path_buf(), asset_id);
            }
        }
        
        Ok(())
    }
    
    /// Register reload callback
    pub fn register_callback(
        &self,
        callback_name: &str,
        asset_name: &str,
        callback: impl Fn(&AssetData) + Send + Sync + 'static,
    ) -> HotReloadResult<()> {
        // Find asset and add callback
        let mut cache = self.cache.write()
            .hot_reload_context("asset_cache")?;
        for (asset_id, cached) in cache.iter_mut() {
            if asset_id.name == asset_name {
                cached.callbacks.push(callback_name.to_string());
                break;
            }
        }
        drop(cache);
        
        // Store callback
        self.callbacks.write()
            .hot_reload_context("callbacks")?
            .insert(
                callback_name.to_string(),
                Box::new(callback),
            );
        Ok(())
    }
    
    /// Handle file change event
    pub fn handle_file_change(&self, event: &WatchEvent) -> HotReloadResult<Vec<String>> {
        match &event.event_type {
            WatchEventType::Modified | WatchEventType::Created => {
                self.reload_asset(&event.path)
            }
            WatchEventType::Deleted => {
                self.remove_asset(&event.path)
            }
            WatchEventType::Renamed { from, to } => {
                self.remove_asset(from)?;
                self.reload_asset(to)
            }
        }
    }
    
    /// Reload asset
    fn reload_asset(&self, path: &Path) -> HotReloadResult<Vec<String>> {
        // Find asset ID
        let asset_id = self.path_map.read()
            .hot_reload_context("path_map")?
            .get(path)
            .cloned();
        
        if let Some(asset_id) = asset_id {
            let mut reloaded_callbacks = Vec::new();
            
            // Reload based on type
            match asset_id.asset_type {
                AssetType::Texture => {
                    // Reload texture
                    let mut image = image::open(path)
                        .map_err(|e| asset_reload_error(&asset_id.name, format!("Failed to reload image: {}", e)))?;
                    
                    // Get GPU texture size limits
                    let device_limits = self.device.limits();
                    let max_texture_dimension = device_limits.max_texture_dimension_2d;
                    
                    // Validate and potentially resize image dimensions
                    let original_dimensions = (image.width(), image.height());
                    let (validated_width, validated_height) = validate_and_resize_image(
                        &mut image,
                        max_texture_dimension,
                        &asset_id.name,
                    );
                    
                    // Log if image was resized during reload
                    if (validated_width, validated_height) != original_dimensions {
                        log::warn!(
                            "[AssetReloader::reload_asset] Image '{}' resized from {}x{} to {}x{} due to GPU texture limit ({}) during hot reload",
                            asset_id.name,
                            original_dimensions.0,
                            original_dimensions.1,
                            validated_width,
                            validated_height,
                            max_texture_dimension
                        );
                    }
                    
                    let rgba = image.to_rgba8();
                    let dimensions = rgba.dimensions();
                    
                    // Get existing texture
                    let texture = {
                        let cache = self.cache.read()
                            .hot_reload_context("asset_cache")?;
                        if let Some(cached) = cache.get(&asset_id) {
                            if let AssetData::Texture(tex_asset) = &cached.data {
                                Some(tex_asset.texture.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };
                    
                    if let Some(texture) = texture {
                        // Check if the existing texture can accommodate the new dimensions
                        let texture_size = texture.size();
                        if dimensions.0 > texture_size.width || dimensions.1 > texture_size.height {
                            // Need to recreate texture with new dimensions
                            log::warn!(
                                "[AssetReloader::reload_asset] Cannot update texture '{}' in-place: new dimensions {}x{} exceed existing texture size {}x{}",
                                asset_id.name,
                                dimensions.0,
                                dimensions.1,
                                texture_size.width,
                                texture_size.height
                            );
                            
                            // Create new texture with validated dimensions
                            let new_texture = self.device.create_texture(&TextureDescriptor {
                                label: Some(&asset_id.name),
                                size: wgpu::Extent3d {
                                    width: dimensions.0,
                                    height: dimensions.1,
                                    depth_or_array_layers: 1,
                                },
                                mip_level_count: 1,
                                sample_count: 1,
                                dimension: wgpu::TextureDimension::D2,
                                format: TextureFormat::Rgba8UnormSrgb,
                                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                                view_formats: &[],
                            });
                            
                            // Upload data to new texture
                            self.queue.write_texture(
                                wgpu::ImageCopyTexture {
                                    texture: &new_texture,
                                    mip_level: 0,
                                    origin: wgpu::Origin3d::ZERO,
                                    aspect: wgpu::TextureAspect::All,
                                },
                                &rgba,
                                wgpu::ImageDataLayout {
                                    offset: 0,
                                    bytes_per_row: Some(4 * dimensions.0),
                                    rows_per_image: Some(dimensions.1),
                                },
                                wgpu::Extent3d {
                                    width: dimensions.0,
                                    height: dimensions.1,
                                    depth_or_array_layers: 1,
                                },
                            );
                            
                            let new_texture = Arc::new(new_texture);
                            let new_view = Arc::new(new_texture.create_view(&wgpu::TextureViewDescriptor::default()));
                            
                            // Update cache with new texture
                            let mut cache = self.cache.write()
                                .hot_reload_context("asset_cache")?;
                            if let Some(cached) = cache.get_mut(&asset_id) {
                                if let AssetData::Texture(tex_asset) = &mut cached.data {
                                    tex_asset.texture = new_texture;
                                    tex_asset.view = new_view;
                                    tex_asset.image = image;
                                }
                                cached.last_modified = std::time::SystemTime::now();
                                reloaded_callbacks = cached.callbacks.clone();
                            }
                        } else {
                            // Dimensions fit in existing texture, update in-place
                            self.queue.write_texture(
                                wgpu::ImageCopyTexture {
                                    texture: &texture,
                                    mip_level: 0,
                                    origin: wgpu::Origin3d::ZERO,
                                    aspect: wgpu::TextureAspect::All,
                                },
                                &rgba,
                                wgpu::ImageDataLayout {
                                    offset: 0,
                                    bytes_per_row: Some(4 * dimensions.0),
                                    rows_per_image: Some(dimensions.1),
                                },
                                wgpu::Extent3d {
                                    width: dimensions.0,
                                    height: dimensions.1,
                                    depth_or_array_layers: 1,
                                },
                            );
                            
                            // Update cache
                            let mut cache = self.cache.write()
                                .hot_reload_context("asset_cache")?;
                            if let Some(cached) = cache.get_mut(&asset_id) {
                                if let AssetData::Texture(tex_asset) = &mut cached.data {
                                    tex_asset.image = image;
                                }
                                cached.last_modified = std::time::SystemTime::now();
                                reloaded_callbacks = cached.callbacks.clone();
                            }
                        }
                    }
                }
                AssetType::Config | AssetType::Script => {
                    // Reload text content
                    let content = std::fs::read_to_string(path)
                        .map_err(|e| EngineError::IoError {
                            path: path.to_string_lossy().to_string(),
                            error: e.to_string(),
                        })?;
                    
                    let mut cache = self.cache.write()
                        .hot_reload_context("asset_cache")?;
                    if let Some(cached) = cache.get_mut(&asset_id) {
                        match asset_id.asset_type {
                            AssetType::Config => cached.data = AssetData::Config(content),
                            AssetType::Script => cached.data = AssetData::Script(content),
                            _ => {}
                        }
                        cached.last_modified = std::time::SystemTime::now();
                        reloaded_callbacks = cached.callbacks.clone();
                    }
                }
                _ => {
                    // Reload binary data
                    let data = std::fs::read(path)
                        .map_err(|e| EngineError::IoError {
                            path: path.to_string_lossy().to_string(),
                            error: e.to_string(),
                        })?;
                    
                    let mut cache = self.cache.write()
                        .hot_reload_context("asset_cache")?;
                    if let Some(cached) = cache.get_mut(&asset_id) {
                        match asset_id.asset_type {
                            AssetType::Model => cached.data = AssetData::Model(data),
                            AssetType::Sound => cached.data = AssetData::Sound(data),
                            _ => {}
                        }
                        cached.last_modified = std::time::SystemTime::now();
                        reloaded_callbacks = cached.callbacks.clone();
                    }
                }
            }
            
            // Trigger callbacks
            self.trigger_callbacks(&asset_id, &reloaded_callbacks)?;
            
            log::info!("Reloaded asset: {} ({:?})", asset_id.name, asset_id.asset_type);
            Ok(reloaded_callbacks)
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Remove asset
    fn remove_asset(&self, path: &Path) -> HotReloadResult<Vec<String>> {
        if let Some(asset_id) = self.path_map.write()
            .hot_reload_context("path_map")?
            .remove(path) 
        {
            self.cache.write()
                .hot_reload_context("asset_cache")?
                .remove(&asset_id);
            log::info!("Removed asset: {}", asset_id.name);
        }
        Ok(Vec::new())
    }
    
    /// Trigger callbacks
    fn trigger_callbacks(
        &self,
        asset_id: &AssetId,
        callback_names: &[String],
    ) -> HotReloadResult<()> {
        let cache = self.cache.read()
            .hot_reload_context("asset_cache")?;
        if let Some(cached) = cache.get(asset_id) {
            let callbacks = self.callbacks.read()
                .hot_reload_context("callbacks")?;
            
            for callback_name in callback_names {
                if let Some(callback) = callbacks.get(callback_name) {
                    callback(&cached.data);
                    log::info!("Triggered callback: {}", callback_name);
                }
            }
        }
        
        Ok(())
    }
    
    /// Get asset
    pub fn get_asset(&self, name: &str, asset_type: AssetType) -> HotReloadResult<Option<AssetData>> {
        let asset_id = AssetId {
            name: name.to_string(),
            asset_type,
        };
        
        Ok(self.cache.read()
            .hot_reload_context("asset_cache")?
            .get(&asset_id)
            .map(|cached| match &cached.data {
                AssetData::Texture(tex) => AssetData::Texture(TextureAsset {
                    texture: tex.texture.clone(),
                    view: tex.view.clone(),
                    image: tex.image.clone(),
                    format: tex.format,
                }),
                AssetData::Model(data) => AssetData::Model(data.clone()),
                AssetData::Sound(data) => AssetData::Sound(data.clone()),
                AssetData::Config(content) => AssetData::Config(content.clone()),
                AssetData::Script(content) => AssetData::Script(content.clone()),
            }))
    }
    
    /// Clear cache
    pub fn clear_cache(&self) -> HotReloadResult<()> {
        self.cache.write()
            .hot_reload_context("asset_cache")?
            .clear();
        self.path_map.write()
            .hot_reload_context("path_map")?
            .clear();
        self.callbacks.write()
            .hot_reload_context("callbacks")?
            .clear();
        Ok(())
    }
}

/// Asset error types
#[derive(Debug)]
pub enum AssetError {
    IoError(std::io::Error),
    LoadError(String),
    UnsupportedType,
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetError::IoError(e) => write!(f, "IO error: {}", e),
            AssetError::LoadError(e) => write!(f, "Load error: {}", e),
            AssetError::UnsupportedType => write!(f, "Unsupported asset type"),
        }
    }
}

impl std::error::Error for AssetError {}

/// Validates image dimensions against GPU limits and resizes if necessary
/// Following DOP principles - pure function that transforms image data
/// Returns (final_width, final_height) after validation/resizing
fn validate_and_resize_image(
    image: &mut DynamicImage,
    max_dimension: u32,
    asset_name: &str,
) -> (u32, u32) {
    let width = image.width();
    let height = image.height();
    
    // Check if dimensions exceed GPU limits
    if width <= max_dimension && height <= max_dimension {
        // Image dimensions are within limits
        return (width, height);
    }
    
    // Calculate scaling factor to fit within limits while maintaining aspect ratio
    let scale_factor = if width > height {
        max_dimension as f32 / width as f32
    } else {
        max_dimension as f32 / height as f32
    };
    
    // Calculate new dimensions
    let new_width = (width as f32 * scale_factor) as u32;
    let new_height = (height as f32 * scale_factor) as u32;
    
    // Ensure dimensions don't exceed limits due to rounding
    let final_width = new_width.min(max_dimension);
    let final_height = new_height.min(max_dimension);
    
    log::info!(
        "[validate_and_resize_image] Resizing image '{}' from {}x{} to {}x{} (scale factor: {:.3})",
        asset_name,
        width,
        height,
        final_width,
        final_height,
        scale_factor
    );
    
    // Resize the image using high-quality filtering
    *image = image.resize_exact(
        final_width,
        final_height,
        image::imageops::FilterType::Lanczos3,
    );
    
    (final_width, final_height)
}