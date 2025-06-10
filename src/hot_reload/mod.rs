/// Hot-Reload System
/// 
/// Enables live updates of code, shaders, and assets without restarting
/// the engine. Designed for rapid iteration during development.
/// 
/// Key features:
/// - Shader recompilation on file change
/// - Asset reloading (textures, models, configs)
/// - Safe state preservation during reloads
/// - Mod development mode with dynamic loading

pub mod watcher;
pub mod shader_reload;
pub mod asset_reload;
pub mod config_reload;
pub mod state_preserve;
pub mod mod_loader;
pub mod rust_reload;

pub use watcher::{FileWatcher, WatchEvent, WatchEventType};
pub use shader_reload::{ShaderReloader, ShaderCache};
pub use asset_reload::{AssetReloader, AssetType};
pub use config_reload::{ConfigReloader, ConfigValue};
pub use state_preserve::{StatePreserver, SerializableState};
pub use mod_loader::{ModLoader, ModInfo};
pub use rust_reload::{RustReloader, HotReloadable};

/// Hot-reload configuration
#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    /// Enable shader hot-reload
    pub shader_reload: bool,
    
    /// Enable asset hot-reload
    pub asset_reload: bool,
    
    /// Enable config hot-reload
    pub config_reload: bool,
    
    /// Enable mod hot-reload
    pub mod_reload: bool,
    
    /// Debounce time in milliseconds
    pub debounce_ms: u64,
    
    /// Directories to watch
    pub watch_dirs: Vec<String>,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            shader_reload: true,
            asset_reload: true,
            config_reload: true,
            mod_reload: false, // Disabled by default for safety
            debounce_ms: 100,
            watch_dirs: vec![
                "src/".to_string(),
                "assets/".to_string(),
                "config/".to_string(),
                "mods/".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests;