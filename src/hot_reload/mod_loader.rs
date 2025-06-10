use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use libloading::{Library, Symbol};
use serde::{Serialize, Deserialize};
use super::{WatchEvent, WatchEventType};

/// Mod metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModInfo {
    /// Mod identifier
    pub id: String,
    
    /// Mod name
    pub name: String,
    
    /// Mod version
    pub version: String,
    
    /// Author
    pub author: String,
    
    /// Description
    pub description: String,
    
    /// Dependencies
    pub dependencies: Vec<String>,
    
    /// Entry point function
    pub entry_point: String,
}

/// Mod API version
pub const MOD_API_VERSION: u32 = 1;

/// Mod interface trait
pub trait ModInterface: Send + Sync {
    /// Initialize mod
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Update mod (called each frame)
    fn update(&mut self, delta_time: f32);
    
    /// Shutdown mod
    fn shutdown(&mut self);
    
    /// Get mod info
    fn info(&self) -> &ModInfo;
}

/// Function types for mod entry points
pub type ModCreateFn = unsafe extern "C" fn() -> *mut dyn ModInterface;
pub type ModDestroyFn = unsafe extern "C" fn(*mut dyn ModInterface);
pub type ModApiVersionFn = unsafe extern "C" fn() -> u32;

/// Loaded mod instance
pub struct LoadedMod {
    /// Mod info
    pub info: ModInfo,
    
    /// Library handle
    library: Library,
    
    /// Mod instance
    instance: *mut dyn ModInterface,
    
    /// Destroy function
    destroy_fn: ModDestroyFn,
    
    /// Load time
    pub load_time: std::time::SystemTime,
}

impl Drop for LoadedMod {
    fn drop(&mut self) {
        // Call destroy function
        unsafe {
            (self.destroy_fn)(self.instance);
        }
    }
}

/// Mod loader
pub struct ModLoader {
    /// Loaded mods
    mods: Arc<RwLock<HashMap<String, LoadedMod>>>,
    
    /// Mod directories
    mod_dirs: Vec<PathBuf>,
    
    /// Temporary directory for hot-reload
    temp_dir: PathBuf,
    
    /// Reload counter for unique names
    reload_counter: Arc<RwLock<u64>>,
}

impl ModLoader {
    /// Create new mod loader
    pub fn new() -> Result<Self, ModError> {
        let temp_dir = std::env::temp_dir().join("earth_engine_mods");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| ModError::IoError(e))?;
        
        Ok(Self {
            mods: Arc::new(RwLock::new(HashMap::new())),
            mod_dirs: vec![PathBuf::from("mods")],
            temp_dir,
            reload_counter: Arc::new(RwLock::new(0)),
        })
    }
    
    /// Add mod directory
    pub fn add_mod_dir(&mut self, path: impl AsRef<Path>) {
        self.mod_dirs.push(path.as_ref().to_path_buf());
    }
    
    /// Load mod from file
    pub fn load_mod(&self, path: impl AsRef<Path>) -> Result<String, ModError> {
        let path = path.as_ref();
        
        // Copy to temp directory for hot-reload
        let counter = {
            let mut c = self.reload_counter.write().unwrap();
            *c += 1;
            *c
        };
        
        let file_name = path.file_name()
            .ok_or(ModError::InvalidPath)?;
        
        let temp_path = self.temp_dir.join(format!("{}_{}", counter, file_name.to_string_lossy()));
        
        std::fs::copy(path, &temp_path)
            .map_err(|e| ModError::IoError(e))?;
        
        // Load library
        let library = unsafe {
            Library::new(&temp_path)
                .map_err(|e| ModError::LoadError(e.to_string()))?
        };
        
        // Check API version
        let api_version: Symbol<ModApiVersionFn> = unsafe {
            library.get(b"mod_api_version\0")
                .map_err(|e| ModError::SymbolError(e.to_string()))?
        };
        
        let version = unsafe { api_version() };
        if version != MOD_API_VERSION {
            return Err(ModError::ApiVersionMismatch(version, MOD_API_VERSION));
        }
        
        // Create mod instance
        let create_fn: Symbol<ModCreateFn> = unsafe {
            library.get(b"mod_create\0")
                .map_err(|e| ModError::SymbolError(e.to_string()))?
        };
        
        let destroy_fn: Symbol<ModDestroyFn> = unsafe {
            library.get(b"mod_destroy\0")
                .map_err(|e| ModError::SymbolError(e.to_string()))?
        };
        
        let instance = unsafe { create_fn() };
        if instance.is_null() {
            return Err(ModError::CreateError);
        }
        
        // Initialize mod
        let mod_interface = unsafe { &mut *instance };
        mod_interface.init()
            .map_err(|e| ModError::InitError(e.to_string()))?;
        
        let info = mod_interface.info().clone();
        let mod_id = info.id.clone();
        
        // Store loaded mod
        let loaded_mod = LoadedMod {
            info,
            library,
            instance,
            destroy_fn: *destroy_fn,
            load_time: std::time::SystemTime::now(),
        };
        
        self.mods.write().unwrap().insert(mod_id.clone(), loaded_mod);
        
        log::info!("Loaded mod: {} v{}", mod_id, mod_interface.info().version);
        Ok(mod_id)
    }
    
    /// Unload mod
    pub fn unload_mod(&self, mod_id: &str) -> Result<(), ModError> {
        let mut mods = self.mods.write().unwrap();
        
        if let Some(mut loaded_mod) = mods.remove(mod_id) {
            // Shutdown mod
            let mod_interface = unsafe { &mut *loaded_mod.instance };
            mod_interface.shutdown();
            
            log::info!("Unloaded mod: {}", mod_id);
            Ok(())
        } else {
            Err(ModError::ModNotFound(mod_id.to_string()))
        }
    }
    
    /// Reload mod
    pub fn reload_mod(&self, mod_id: &str, path: impl AsRef<Path>) -> Result<(), ModError> {
        // Unload existing
        self.unload_mod(mod_id)?;
        
        // Load new version
        self.load_mod(path)?;
        
        Ok(())
    }
    
    /// Update all mods
    pub fn update_mods(&self, delta_time: f32) {
        let mods = self.mods.read().unwrap();
        
        for (_, loaded_mod) in mods.iter() {
            let mod_interface = unsafe { &mut *loaded_mod.instance };
            mod_interface.update(delta_time);
        }
    }
    
    /// Get loaded mod
    pub fn get_mod(&self, mod_id: &str) -> Option<&dyn ModInterface> {
        self.mods.read().unwrap()
            .get(mod_id)
            .map(|loaded| unsafe { &*loaded.instance })
    }
    
    /// List loaded mods
    pub fn list_mods(&self) -> Vec<ModInfo> {
        self.mods.read().unwrap()
            .values()
            .map(|loaded| loaded.info.clone())
            .collect()
    }
    
    /// Handle file change event
    pub fn handle_file_change(&self, event: &WatchEvent) -> Result<(), ModError> {
        // Check if this is a mod file
        if !matches!(
            event.path.extension().and_then(|e| e.to_str()),
            Some("dll") | Some("so") | Some("dylib")
        ) {
            return Ok(());
        }
        
        match &event.event_type {
            WatchEventType::Modified => {
                // Find mod using this file
                let mods = self.mods.read().unwrap();
                for (mod_id, _) in mods.iter() {
                    // In practice, would track original paths
                    log::info!("Mod file changed, consider reloading: {}", mod_id);
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Scan directories for mods
    pub fn scan_mods(&self) -> Vec<PathBuf> {
        let mut mod_files = Vec::new();
        
        for dir in &self.mod_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension() {
                            let ext_str = ext.to_string_lossy();
                            if matches!(ext_str.as_ref(), "dll" | "so" | "dylib") {
                                mod_files.push(path);
                            }
                        }
                    }
                }
            }
        }
        
        mod_files
    }
}

/// Mod error types
#[derive(Debug)]
pub enum ModError {
    IoError(std::io::Error),
    LoadError(String),
    SymbolError(String),
    ApiVersionMismatch(u32, u32),
    CreateError,
    InitError(String),
    ModNotFound(String),
    InvalidPath,
}

impl std::fmt::Display for ModError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModError::IoError(e) => write!(f, "IO error: {}", e),
            ModError::LoadError(e) => write!(f, "Load error: {}", e),
            ModError::SymbolError(e) => write!(f, "Symbol error: {}", e),
            ModError::ApiVersionMismatch(got, expected) => {
                write!(f, "API version mismatch: got {}, expected {}", got, expected)
            }
            ModError::CreateError => write!(f, "Failed to create mod instance"),
            ModError::InitError(e) => write!(f, "Init error: {}", e),
            ModError::ModNotFound(id) => write!(f, "Mod not found: {}", id),
            ModError::InvalidPath => write!(f, "Invalid mod path"),
        }
    }
}

impl std::error::Error for ModError {}

/// Example mod implementation (for testing)
#[cfg(test)]
pub mod example {
    use super::*;
    
    pub struct ExampleMod {
        info: ModInfo,
        counter: u32,
    }
    
    impl ExampleMod {
        pub fn new() -> Self {
            Self {
                info: ModInfo {
                    id: "example_mod".to_string(),
                    name: "Example Mod".to_string(),
                    version: "1.0.0".to_string(),
                    author: "Test Author".to_string(),
                    description: "An example mod for testing".to_string(),
                    dependencies: vec![],
                    entry_point: "mod_create".to_string(),
                },
                counter: 0,
            }
        }
    }
    
    impl ModInterface for ExampleMod {
        fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            log::info!("Example mod initialized");
            Ok(())
        }
        
        fn update(&mut self, delta_time: f32) {
            self.counter += 1;
            if self.counter % 60 == 0 {
                log::debug!("Example mod update: {}", delta_time);
            }
        }
        
        fn shutdown(&mut self) {
            log::info!("Example mod shutdown");
        }
        
        fn info(&self) -> &ModInfo {
            &self.info
        }
    }
    
    #[no_mangle]
    pub extern "C" fn mod_api_version() -> u32 {
        MOD_API_VERSION
    }
    
    #[no_mangle]
    pub extern "C" fn mod_create() -> *mut dyn ModInterface {
        let mod_instance = Box::new(ExampleMod::new());
        Box::into_raw(mod_instance) as *mut dyn ModInterface
    }
    
    #[no_mangle]
    pub unsafe extern "C" fn mod_destroy(instance: *mut dyn ModInterface) {
        if !instance.is_null() {
            let _ = Box::from_raw(instance);
        }
    }
}