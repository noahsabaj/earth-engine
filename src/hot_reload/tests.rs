#[cfg(test)]
mod tests {
    use super::*;
    use crate::hot_reload::*;
    use tempfile::TempDir;
    use std::fs;
    use std::time::Duration;
    use std::sync::Arc;
    
    #[test]
    fn test_hot_reload_config() {
        let config = HotReloadConfig::default();
        assert!(config.shader_reload);
        assert!(config.asset_reload);
        assert!(config.config_reload);
        assert!(!config.mod_reload); // Disabled by default
        assert_eq!(config.debounce_ms, 100);
    }
    
    #[test]
    fn test_file_filter() {
        let filter = FileFilter::new(vec!["wgsl", "glsl"]);
        
        assert!(filter.matches(std::path::Path::new("shader.wgsl")));
        assert!(filter.matches(std::path::Path::new("vertex.glsl")));
        assert!(!filter.matches(std::path::Path::new("texture.png")));
        assert!(!filter.matches(std::path::Path::new("no_extension")));
    }
    
    #[test]
    fn test_event_batcher() {
        let mut batcher = EventBatcher::new(100);
        
        let event1 = WatchEvent {
            path: std::path::PathBuf::from("file1.txt"),
            event_type: WatchEventType::Modified,
            timestamp: std::time::Instant::now(),
        };
        
        let event2 = WatchEvent {
            path: std::path::PathBuf::from("file2.txt"),
            event_type: WatchEventType::Created,
            timestamp: std::time::Instant::now(),
        };
        
        batcher.add_event(event1);
        batcher.add_event(event2);
        
        // Should not get batch immediately
        assert!(batcher.get_batch().is_none());
        
        // Force batch
        let batch = batcher.force_batch();
        assert_eq!(batch.len(), 2);
    }
    
    #[test]
    fn test_config_value() {
        let bool_val = ConfigValue::Bool(true);
        assert_eq!(bool_val.as_bool(), Some(true));
        assert_eq!(bool_val.as_i64(), None);
        
        let int_val = ConfigValue::Integer(42);
        assert_eq!(int_val.as_i64(), Some(42));
        assert_eq!(int_val.as_f64(), Some(42.0));
        
        let str_val = ConfigValue::String("hello".to_string());
        assert_eq!(str_val.as_str(), Some("hello"));
        
        let array_val = ConfigValue::Array(vec![
            ConfigValue::Integer(1),
            ConfigValue::Integer(2),
        ]);
        assert!(array_val.as_array().is_some());
        assert_eq!(array_val.as_array().unwrap().len(), 2);
    }
    
    #[test]
    fn test_config_format_detection() {
        assert_eq!(
            ConfigFormat::from_path(std::path::Path::new("config.json")),
            Some(ConfigFormat::Json)
        );
        assert_eq!(
            ConfigFormat::from_path(std::path::Path::new("settings.toml")),
            Some(ConfigFormat::Toml)
        );
        assert_eq!(
            ConfigFormat::from_path(std::path::Path::new("data.yaml")),
            Some(ConfigFormat::Yaml)
        );
        assert_eq!(
            ConfigFormat::from_path(std::path::Path::new("game.ron")),
            Some(ConfigFormat::Ron)
        );
        assert_eq!(
            ConfigFormat::from_path(std::path::Path::new("unknown.txt")),
            None
        );
    }
    
    #[test]
    fn test_asset_type_detection() {
        assert_eq!(
            AssetType::from_path(std::path::Path::new("texture.png")),
            AssetType::Texture
        );
        assert_eq!(
            AssetType::from_path(std::path::Path::new("model.gltf")),
            AssetType::Model
        );
        assert_eq!(
            AssetType::from_path(std::path::Path::new("sound.wav")),
            AssetType::Sound
        );
        assert_eq!(
            AssetType::from_path(std::path::Path::new("config.json")),
            AssetType::Config
        );
        assert_eq!(
            AssetType::from_path(std::path::Path::new("script.lua")),
            AssetType::Script
        );
        assert_eq!(
            AssetType::from_path(std::path::Path::new("unknown.xyz")),
            AssetType::Unknown
        );
    }
    
    #[test]
    fn test_state_preservation() {
        use crate::hot_reload::state_preserve::*;
        
        let preserver = StatePreserver::new(10);
        
        // Create test state
        let player_state = PlayerState {
            position: [10.0, 20.0, 30.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            health: 100.0,
            inventory: vec![1, 2, 3],
        };
        
        preserver.register_state(Box::new(player_state));
        
        // Create snapshot
        let snapshots = preserver.create_snapshot().unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, "player");
        
        // Verify snapshot data is not empty
        assert!(!snapshots[0].data.is_empty());
    }
    
    #[test]
    fn test_config_builder() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.json");
        
        // Write test config
        fs::write(&config_path, r#"{"key": "value"}"#).unwrap();
        
        let reloader = ConfigBuilder::new()
            .add_config("test", &config_path)
            .add_default("default_key", ConfigValue::String("default".to_string()))
            .build()
            .unwrap();
        
        // Test getting values
        assert_eq!(
            reloader.get("test", "key").ok().flatten().and_then(|v| v.as_str().map(|s| s.to_string())),
            Some("value".to_string())
        );
        assert_eq!(
            reloader.get("test", "default_key").ok().flatten().and_then(|v| v.as_str().map(|s| s.to_string())),
            Some("default".to_string())
        );
    }
    
    #[tokio::test]
    async fn test_shader_cache() {
        let (device, _queue) = create_test_device().await;
        let device = std::sync::Arc::new(device);
        
        let reloader = std::sync::Arc::new(ShaderReloader::new(device.clone()));
        let cache = ShaderCache::new(reloader.clone());
        
        // Test loading shader from source
        let shader = reloader.load_shader_source(
            "test_shader",
            "@compute @workgroup_size(1) fn main() {}"
        );
        
        // Should be cached
        assert!(reloader.get_shader("test_shader").is_ok());
    }
    
    #[test]
    fn test_mod_info() {
        let info = ModInfo {
            id: "test_mod".to_string(),
            name: "Test Mod".to_string(),
            version: "1.0.0".to_string(),
            author: "Test Author".to_string(),
            description: "A test mod".to_string(),
            dependencies: vec!["core".to_string()],
            entry_point: "mod_init".to_string(),
        };
        
        assert_eq!(info.id, "test_mod");
        assert_eq!(info.dependencies.len(), 1);
    }
    
    #[test]
    fn test_rust_reloader() {
        let mut reloader = RustReloader::new();
        
        // Test that enable_watch doesn't panic
        reloader.enable_watch();
        // Watch enabled successfully if no panic occurs
        
        // Test file detection
        assert!(RustReloader::is_rust_file(std::path::Path::new("main.rs")));
        assert!(!RustReloader::is_rust_file(std::path::Path::new("main.txt")));
    }
    
    // Helper function to create test device
    async fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: true,
            })
            .await
            .expect("Failed to find adapter");
        
        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Test Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .expect("Failed to create device")
    }
}