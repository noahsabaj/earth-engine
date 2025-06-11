use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use wgpu::{Device, ShaderModule, ShaderModuleDescriptor, ShaderSource};
use super::{FileWatcher, WatchEvent, WatchEventType, HotReloadResult, HotReloadErrorContext, shader_reload_error};
use crate::error::EngineError;

/// Shader type identifier
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ShaderId {
    /// Unique name for the shader
    pub name: String,
    
    /// File path (if loaded from file)
    pub path: Option<PathBuf>,
}

/// Cached shader data
#[derive(Clone)]
pub struct CachedShader {
    /// Shader module
    pub module: Arc<ShaderModule>,
    
    /// Source code
    pub source: String,
    
    /// Last modified time
    pub last_modified: std::time::SystemTime,
    
    /// Dependent pipelines
    pub dependents: Vec<String>,
}

/// Shader hot-reload manager
pub struct ShaderReloader {
    /// Device reference
    device: Arc<Device>,
    
    /// Shader cache
    cache: Arc<RwLock<HashMap<ShaderId, CachedShader>>>,
    
    /// Path to shader ID mapping
    path_map: Arc<RwLock<HashMap<PathBuf, ShaderId>>>,
    
    /// Pipeline rebuild callbacks
    rebuild_callbacks: Arc<RwLock<HashMap<String, Box<dyn Fn(&Device, &ShaderModule) + Send + Sync>>>>,
    
    /// Include paths for shader imports
    include_paths: Vec<PathBuf>,
}

impl ShaderReloader {
    /// Create new shader reloader
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            device,
            cache: Arc::new(RwLock::new(HashMap::new())),
            path_map: Arc::new(RwLock::new(HashMap::new())),
            rebuild_callbacks: Arc::new(RwLock::new(HashMap::new())),
            include_paths: vec![PathBuf::from("src/shaders")],
        }
    }
    
    /// Add include path for shader imports
    pub fn add_include_path(&mut self, path: impl AsRef<Path>) {
        self.include_paths.push(path.as_ref().to_path_buf());
    }
    
    /// Load shader from file
    pub fn load_shader(
        &self,
        name: &str,
        path: impl AsRef<Path>,
    ) -> HotReloadResult<Arc<ShaderModule>> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path)
            .map_err(|e| EngineError::IoError {
                path: path.to_string_lossy().to_string(),
                error: e.to_string(),
            })?;
        
        // Process includes
        let processed_source = self.process_includes(&source, path)?;
        
        // Create shader module
        let module = self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some(name),
            source: ShaderSource::Wgsl(processed_source.clone().into()),
        });
        
        let module = Arc::new(module);
        
        // Cache shader
        let shader_id = ShaderId {
            name: name.to_string(),
            path: Some(path.to_path_buf()),
        };
        
        let cached = CachedShader {
            module: module.clone(),
            source: processed_source,
            last_modified: std::fs::metadata(path)
                .and_then(|m| m.modified().ok())
                .unwrap_or(std::time::SystemTime::now()),
            dependents: Vec::new(),
        };
        
        self.cache.write()
            .hot_reload_context("shader_cache")?
            .insert(shader_id.clone(), cached);
        self.path_map.write()
            .hot_reload_context("path_map")?
            .insert(path.to_path_buf(), shader_id);
        
        Ok(module)
    }
    
    /// Load shader from source
    pub fn load_shader_source(
        &self,
        name: &str,
        source: &str,
    ) -> HotReloadResult<Arc<ShaderModule>> {
        let module = self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some(name),
            source: ShaderSource::Wgsl(source.into()),
        });
        
        let module = Arc::new(module);
        
        // Cache shader
        let shader_id = ShaderId {
            name: name.to_string(),
            path: None,
        };
        
        let cached = CachedShader {
            module: module.clone(),
            source: source.to_string(),
            last_modified: std::time::SystemTime::now(),
            dependents: Vec::new(),
        };
        
        self.cache.write()
            .hot_reload_context("shader_cache")?
            .insert(shader_id, cached);
        
        Ok(module)
    }
    
    /// Register pipeline rebuild callback
    pub fn register_pipeline(
        &self,
        pipeline_name: &str,
        shader_name: &str,
        rebuild_fn: impl Fn(&Device, &ShaderModule) + Send + Sync + 'static,
    ) -> HotReloadResult<()> {
        // Add to dependents
        if let Some(shader) = self.cache.write()
            .hot_reload_context("shader_cache")?
            .get_mut(&ShaderId {
                name: shader_name.to_string(),
                path: None,
            }) 
        {
            shader.dependents.push(pipeline_name.to_string());
        }
        
        // Store callback
        self.rebuild_callbacks.write()
            .hot_reload_context("rebuild_callbacks")?
            .insert(
                pipeline_name.to_string(),
                Box::new(rebuild_fn),
            );
        Ok(())
    }
    
    /// Process file change event
    pub fn handle_file_change(&self, event: &WatchEvent) -> HotReloadResult<Vec<String>> {
        match &event.event_type {
            WatchEventType::Modified | WatchEventType::Created => {
                self.reload_shader(&event.path)
            }
            WatchEventType::Deleted => {
                self.remove_shader(&event.path)
            }
            WatchEventType::Renamed { from, to } => {
                self.remove_shader(from)?;
                self.reload_shader(to)
            }
        }
    }
    
    /// Reload shader from file
    fn reload_shader(&self, path: &Path) -> HotReloadResult<Vec<String>> {
        // Check if this is a shader file
        if !matches!(path.extension().and_then(|e| e.to_str()), Some("wgsl") | Some("glsl")) {
            return Ok(Vec::new());
        }
        
        // Find shader ID
        let shader_id = self.path_map.read()
            .hot_reload_context("path_map")?
            .get(path)
            .cloned();
        
        if let Some(shader_id) = shader_id {
            // Read new source
            let source = std::fs::read_to_string(path)
                .map_err(|e| EngineError::IoError {
                    path: path.to_string_lossy().to_string(),
                    error: e.to_string(),
                })?;
            
            // Process includes
            let processed_source = self.process_includes(&source, path)?;
            
            // Try to compile new shader
            let new_module = match self.try_compile_shader(&shader_id.name, &processed_source) {
                Ok(module) => module,
                Err(e) => {
                    log::error!("Shader compilation failed: {:?}", e);
                    return Err(e);
                }
            };
            
            // Update cache
            let mut rebuilt_pipelines = Vec::new();
            
            {
                let mut cache = self.cache.write()
                .hot_reload_context("shader_cache")?;
                if let Some(cached) = cache.get_mut(&shader_id) {
                    cached.module = Arc::new(new_module);
                    cached.source = processed_source;
                    cached.last_modified = std::fs::metadata(path)
                        .map(|m| m.modified().unwrap_or(std::time::SystemTime::now()))
                        .unwrap_or(std::time::SystemTime::now());
                    
                    rebuilt_pipelines = cached.dependents.clone();
                }
            }
            
            // Rebuild dependent pipelines
            self.rebuild_pipelines(&rebuilt_pipelines, &shader_id)?;
            
            log::info!("Reloaded shader: {}", shader_id.name);
            Ok(rebuilt_pipelines)
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Remove shader from cache
    fn remove_shader(&self, path: &Path) -> HotReloadResult<Vec<String>> {
        if let Some(shader_id) = self.path_map.write()
            .hot_reload_context("path_map")?
            .remove(path) 
        {
            self.cache.write()
                .hot_reload_context("shader_cache")?
                .remove(&shader_id);
            log::info!("Removed shader: {}", shader_id.name);
        }
        Ok(Vec::new())
    }
    
    /// Try to compile shader
    fn try_compile_shader(&self, name: &str, source: &str) -> HotReloadResult<ShaderModule> {
        // Note: wgpu doesn't provide validation without creating the module
        // In a real implementation, we might use naga for validation
        Ok(self.device.create_shader_module(ShaderModuleDescriptor {
            label: Some(name),
            source: ShaderSource::Wgsl(source.into()),
        }))
    }
    
    /// Rebuild dependent pipelines
    fn rebuild_pipelines(
        &self,
        pipeline_names: &[String],
        shader_id: &ShaderId,
    ) -> HotReloadResult<()> {
        let cache = self.cache.read()
            .hot_reload_context("shader_cache")?;
        if let Some(cached) = cache.get(shader_id) {
            let callbacks = self.rebuild_callbacks.read()
                .hot_reload_context("rebuild_callbacks")?;
            
            for pipeline_name in pipeline_names {
                if let Some(callback) = callbacks.get(pipeline_name) {
                    callback(&self.device, &cached.module);
                    log::info!("Rebuilt pipeline: {}", pipeline_name);
                }
            }
        }
        
        Ok(())
    }
    
    /// Process shader includes
    fn process_includes(&self, source: &str, base_path: &Path) -> HotReloadResult<String> {
        let mut processed = String::new();
        let base_dir = base_path.parent().unwrap_or(Path::new("."));
        
        for line in source.lines() {
            if let Some(include_path) = line.trim().strip_prefix("#include") {
                let include_path = include_path.trim().trim_matches('"');
                
                // Try to find include file
                let mut found = false;
                
                // First try relative to current file
                let relative_path = base_dir.join(include_path);
                if relative_path.exists() {
                    let include_source = std::fs::read_to_string(&relative_path)
                        .map_err(|e| EngineError::IoError {
                            path: relative_path.to_string_lossy().to_string(),
                            error: e.to_string(),
                        })?;
                    processed.push_str(&self.process_includes(&include_source, &relative_path)?);
                    found = true;
                } else {
                    // Try include paths
                    for include_dir in &self.include_paths {
                        let full_path = include_dir.join(include_path);
                        if full_path.exists() {
                            let include_source = std::fs::read_to_string(&full_path)
                                .map_err(|e| EngineError::IoError {
                                    path: full_path.to_string_lossy().to_string(),
                                    error: e.to_string(),
                                })?;
                            processed.push_str(&self.process_includes(&include_source, &full_path)?);
                            found = true;
                            break;
                        }
                    }
                }
                
                if !found {
                    return Err(shader_reload_error("shader", format!("Include not found: {}", include_path)));
                }
            } else {
                processed.push_str(line);
                processed.push('\n');
            }
        }
        
        Ok(processed)
    }
    
    /// Get cached shader
    pub fn get_shader(&self, name: &str) -> HotReloadResult<Option<Arc<ShaderModule>>> {
        let shader_id = ShaderId {
            name: name.to_string(),
            path: None,
        };
        
        Ok(self.cache.read()
            .hot_reload_context("shader_cache")?
            .get(&shader_id)
            .map(|cached| cached.module.clone()))
    }
    
    /// Clear shader cache
    pub fn clear_cache(&self) -> HotReloadResult<()> {
        self.cache.write()
            .hot_reload_context("shader_cache")?
            .clear();
        self.path_map.write()
            .hot_reload_context("path_map")?
            .clear();
        self.rebuild_callbacks.write()
            .hot_reload_context("rebuild_callbacks")?
            .clear();
        Ok(())
    }
}

/// Shader cache for quick access
pub struct ShaderCache {
    reloader: Arc<ShaderReloader>,
}

impl ShaderCache {
    /// Create new shader cache
    pub fn new(reloader: Arc<ShaderReloader>) -> Self {
        Self { reloader }
    }
    
    /// Get or load shader
    pub fn get_or_load(
        &self,
        name: &str,
        path: impl AsRef<Path>,
    ) -> HotReloadResult<Arc<ShaderModule>> {
        if let Some(shader) = self.reloader.get_shader(name)? {
            Ok(shader)
        } else {
            self.reloader.load_shader(name, path)
        }
    }
}

/// Shader error types
#[derive(Debug)]
pub enum ShaderError {
    IoError(std::io::Error),
    CompilationError(String),
    IncludeNotFound(String),
}

impl std::fmt::Display for ShaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShaderError::IoError(e) => write!(f, "IO error: {}", e),
            ShaderError::CompilationError(e) => write!(f, "Compilation error: {}", e),
            ShaderError::IncludeNotFound(path) => write!(f, "Include not found: {}", path),
        }
    }
}

impl std::error::Error for ShaderError {}