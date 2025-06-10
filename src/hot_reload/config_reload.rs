use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use toml::Value as TomlValue;
use super::{WatchEvent, WatchEventType};

/// Configuration value type
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Array(Vec<ConfigValue>),
    Object(HashMap<String, ConfigValue>),
}

impl ConfigValue {
    /// Get as bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Bool(v) => Some(*v),
            _ => None,
        }
    }
    
    /// Get as integer
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ConfigValue::Integer(v) => Some(*v),
            _ => None,
        }
    }
    
    /// Get as float
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(v) => Some(*v),
            ConfigValue::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }
    
    /// Get as string
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::String(v) => Some(v),
            _ => None,
        }
    }
    
    /// Get as array
    pub fn as_array(&self) -> Option<&Vec<ConfigValue>> {
        match self {
            ConfigValue::Array(v) => Some(v),
            _ => None,
        }
    }
    
    /// Get as object
    pub fn as_object(&self) -> Option<&HashMap<String, ConfigValue>> {
        match self {
            ConfigValue::Object(v) => Some(v),
            _ => None,
        }
    }
}

/// Configuration format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigFormat {
    Json,
    Toml,
    Yaml,
    Ron,
}

impl ConfigFormat {
    /// Detect format from file extension
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("json") => Some(ConfigFormat::Json),
            Some("toml") => Some(ConfigFormat::Toml),
            Some("yaml") | Some("yml") => Some(ConfigFormat::Yaml),
            Some("ron") => Some(ConfigFormat::Ron),
            _ => None,
        }
    }
}

/// Configuration file data
#[derive(Clone)]
pub struct ConfigFile {
    /// File path
    pub path: PathBuf,
    
    /// Configuration format
    pub format: ConfigFormat,
    
    /// Parsed configuration
    pub data: ConfigValue,
    
    /// Raw content
    pub raw: String,
    
    /// Last modified time
    pub last_modified: std::time::SystemTime,
    
    /// Change callbacks
    pub callbacks: Vec<String>,
}

/// Configuration reloader
pub struct ConfigReloader {
    /// Configuration cache
    cache: Arc<RwLock<HashMap<String, ConfigFile>>>,
    
    /// Path to config name mapping
    path_map: Arc<RwLock<HashMap<PathBuf, String>>>,
    
    /// Change callbacks
    callbacks: Arc<RwLock<HashMap<String, Box<dyn Fn(&ConfigValue) + Send + Sync>>>>,
    
    /// Default values
    defaults: Arc<RwLock<HashMap<String, ConfigValue>>>,
}

impl ConfigReloader {
    /// Create new config reloader
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            path_map: Arc::new(RwLock::new(HashMap::new())),
            callbacks: Arc::new(RwLock::new(HashMap::new())),
            defaults: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Load configuration file
    pub fn load_config(
        &self,
        name: &str,
        path: impl AsRef<Path>,
    ) -> Result<ConfigValue, ConfigError> {
        let path = path.as_ref();
        
        // Detect format
        let format = ConfigFormat::from_path(path)
            .ok_or_else(|| ConfigError::UnknownFormat)?;
        
        // Read file
        let raw = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::IoError(e))?;
        
        // Parse based on format
        let data = match format {
            ConfigFormat::Json => self.parse_json(&raw)?,
            ConfigFormat::Toml => self.parse_toml(&raw)?,
            ConfigFormat::Yaml => self.parse_yaml(&raw)?,
            ConfigFormat::Ron => self.parse_ron(&raw)?,
        };
        
        // Cache config
        let config_file = ConfigFile {
            path: path.to_path_buf(),
            format,
            data: data.clone(),
            raw,
            last_modified: std::fs::metadata(path)
                .map(|m| m.modified().unwrap_or(std::time::SystemTime::now()))
                .unwrap_or(std::time::SystemTime::now()),
            callbacks: Vec::new(),
        };
        
        self.cache.write().unwrap().insert(name.to_string(), config_file);
        self.path_map.write().unwrap().insert(path.to_path_buf(), name.to_string());
        
        Ok(data)
    }
    
    /// Register config change callback
    pub fn register_callback(
        &self,
        callback_name: &str,
        config_name: &str,
        callback: impl Fn(&ConfigValue) + Send + Sync + 'static,
    ) {
        // Add to config callbacks
        if let Some(config) = self.cache.write().unwrap().get_mut(config_name) {
            config.callbacks.push(callback_name.to_string());
        }
        
        // Store callback
        self.callbacks.write().unwrap().insert(
            callback_name.to_string(),
            Box::new(callback),
        );
    }
    
    /// Set default value
    pub fn set_default(&self, key: &str, value: ConfigValue) {
        self.defaults.write().unwrap().insert(key.to_string(), value);
    }
    
    /// Get config value
    pub fn get(&self, config_name: &str, key: &str) -> Option<ConfigValue> {
        let cache = self.cache.read().unwrap();
        
        if let Some(config) = cache.get(config_name) {
            self.get_nested_value(&config.data, key)
        } else {
            self.defaults.read().unwrap().get(key).cloned()
        }
    }
    
    /// Get config value with default
    pub fn get_or(&self, config_name: &str, key: &str, default: ConfigValue) -> ConfigValue {
        self.get(config_name, key).unwrap_or(default)
    }
    
    /// Handle file change event
    pub fn handle_file_change(&self, event: &WatchEvent) -> Result<Vec<String>, ConfigError> {
        match &event.event_type {
            WatchEventType::Modified | WatchEventType::Created => {
                self.reload_config(&event.path)
            }
            WatchEventType::Deleted => {
                self.remove_config(&event.path)
            }
            WatchEventType::Renamed { from, to } => {
                self.remove_config(from)?;
                self.reload_config(to)
            }
        }
    }
    
    /// Reload configuration
    fn reload_config(&self, path: &Path) -> Result<Vec<String>, ConfigError> {
        // Find config name
        let config_name = self.path_map.read().unwrap().get(path).cloned();
        
        if let Some(config_name) = config_name {
            // Read and parse new content
            let raw = std::fs::read_to_string(path)
                .map_err(|e| ConfigError::IoError(e))?;
            
            let format = {
                let cache = self.cache.read().unwrap();
                cache.get(&config_name).map(|c| c.format)
            }.ok_or(ConfigError::NotFound)?;
            
            // Parse based on format
            let new_data = match format {
                ConfigFormat::Json => self.parse_json(&raw)?,
                ConfigFormat::Toml => self.parse_toml(&raw)?,
                ConfigFormat::Yaml => self.parse_yaml(&raw)?,
                ConfigFormat::Ron => self.parse_ron(&raw)?,
            };
            
            // Get callbacks before updating
            let callbacks = {
                let cache = self.cache.read().unwrap();
                cache.get(&config_name).map(|c| c.callbacks.clone()).unwrap_or_default()
            };
            
            // Update cache
            {
                let mut cache = self.cache.write().unwrap();
                if let Some(config) = cache.get_mut(&config_name) {
                    config.data = new_data.clone();
                    config.raw = raw;
                    config.last_modified = std::time::SystemTime::now();
                }
            }
            
            // Trigger callbacks
            self.trigger_callbacks(&new_data, &callbacks)?;
            
            log::info!("Reloaded config: {}", config_name);
            Ok(callbacks)
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Remove configuration
    fn remove_config(&self, path: &Path) -> Result<Vec<String>, ConfigError> {
        if let Some(config_name) = self.path_map.write().unwrap().remove(path) {
            self.cache.write().unwrap().remove(&config_name);
            log::info!("Removed config: {}", config_name);
        }
        Ok(Vec::new())
    }
    
    /// Trigger callbacks
    fn trigger_callbacks(
        &self,
        data: &ConfigValue,
        callback_names: &[String],
    ) -> Result<(), ConfigError> {
        let callbacks = self.callbacks.read().unwrap();
        
        for callback_name in callback_names {
            if let Some(callback) = callbacks.get(callback_name) {
                callback(data);
                log::info!("Triggered config callback: {}", callback_name);
            }
        }
        
        Ok(())
    }
    
    /// Parse JSON
    fn parse_json(&self, raw: &str) -> Result<ConfigValue, ConfigError> {
        let json: JsonValue = serde_json::from_str(raw)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;
        
        Ok(self.json_to_config_value(json))
    }
    
    /// Parse TOML
    fn parse_toml(&self, raw: &str) -> Result<ConfigValue, ConfigError> {
        let toml: TomlValue = toml::from_str(raw)
            .map_err(|e| ConfigError::ParseError(e.to_string()))?;
        
        Ok(self.toml_to_config_value(toml))
    }
    
    /// Parse YAML (placeholder)
    fn parse_yaml(&self, _raw: &str) -> Result<ConfigValue, ConfigError> {
        // Would use serde_yaml
        Err(ConfigError::UnknownFormat)
    }
    
    /// Parse RON (placeholder)
    fn parse_ron(&self, _raw: &str) -> Result<ConfigValue, ConfigError> {
        // Would use ron
        Err(ConfigError::UnknownFormat)
    }
    
    /// Convert JSON value to config value
    fn json_to_config_value(&self, json: JsonValue) -> ConfigValue {
        match json {
            JsonValue::Null => ConfigValue::String("null".to_string()),
            JsonValue::Bool(v) => ConfigValue::Bool(v),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    ConfigValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    ConfigValue::Float(f)
                } else {
                    ConfigValue::Float(0.0)
                }
            }
            JsonValue::String(s) => ConfigValue::String(s),
            JsonValue::Array(arr) => {
                ConfigValue::Array(arr.into_iter().map(|v| self.json_to_config_value(v)).collect())
            }
            JsonValue::Object(obj) => {
                ConfigValue::Object(obj.into_iter().map(|(k, v)| (k, self.json_to_config_value(v))).collect())
            }
        }
    }
    
    /// Convert TOML value to config value
    fn toml_to_config_value(&self, toml: TomlValue) -> ConfigValue {
        match toml {
            TomlValue::Boolean(v) => ConfigValue::Bool(v),
            TomlValue::Integer(v) => ConfigValue::Integer(v),
            TomlValue::Float(v) => ConfigValue::Float(v),
            TomlValue::String(s) => ConfigValue::String(s),
            TomlValue::Array(arr) => {
                ConfigValue::Array(arr.into_iter().map(|v| self.toml_to_config_value(v)).collect())
            }
            TomlValue::Table(table) => {
                ConfigValue::Object(table.into_iter().map(|(k, v)| (k, self.toml_to_config_value(v))).collect())
            }
            _ => ConfigValue::String("unsupported".to_string()),
        }
    }
    
    /// Get nested value by key path (e.g., "player.health.max")
    fn get_nested_value(&self, data: &ConfigValue, key: &str) -> Option<ConfigValue> {
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = data;
        
        for part in parts {
            match current {
                ConfigValue::Object(map) => {
                    current = map.get(part)?;
                }
                _ => return None,
            }
        }
        
        Some(current.clone())
    }
}

/// Configuration error types
#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    ParseError(String),
    UnknownFormat,
    NotFound,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::ParseError(e) => write!(f, "Parse error: {}", e),
            ConfigError::UnknownFormat => write!(f, "Unknown configuration format"),
            ConfigError::NotFound => write!(f, "Configuration not found"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Configuration builder for easy setup
pub struct ConfigBuilder {
    configs: Vec<(String, PathBuf)>,
    defaults: HashMap<String, ConfigValue>,
}

impl ConfigBuilder {
    /// Create new config builder
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
            defaults: HashMap::new(),
        }
    }
    
    /// Add configuration file
    pub fn add_config(mut self, name: &str, path: impl AsRef<Path>) -> Self {
        self.configs.push((name.to_string(), path.as_ref().to_path_buf()));
        self
    }
    
    /// Add default value
    pub fn add_default(mut self, key: &str, value: ConfigValue) -> Self {
        self.defaults.insert(key.to_string(), value);
        self
    }
    
    /// Build config reloader
    pub fn build(self) -> Result<ConfigReloader, ConfigError> {
        let reloader = ConfigReloader::new();
        
        // Set defaults
        for (key, value) in self.defaults {
            reloader.set_default(&key, value);
        }
        
        // Load configs
        for (name, path) in self.configs {
            reloader.load_config(&name, path)?;
        }
        
        Ok(reloader)
    }
}