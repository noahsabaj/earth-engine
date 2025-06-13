use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use notify::{Watcher, RecursiveMode, Event, EventKind, RecommendedWatcher};
use tokio::sync::mpsc;
use super::{HotReloadResult, HotReloadErrorContext};

/// Type of file change event
#[derive(Debug, Clone, PartialEq)]
pub enum WatchEventType {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf, to: PathBuf },
}

/// File watch event
#[derive(Debug, Clone)]
pub struct WatchEvent {
    pub path: PathBuf,
    pub event_type: WatchEventType,
    pub timestamp: Instant,
}

/// File watcher with debouncing
pub struct FileWatcher {
    /// Notify watcher instance
    watcher: RecommendedWatcher,
    
    /// Event receiver
    rx: mpsc::UnboundedReceiver<WatchEvent>,
    
    /// Debounce map
    debounce_map: Arc<Mutex<HashMap<PathBuf, Instant>>>,
    
    /// Debounce duration
    debounce_duration: Duration,
    
    /// Active watch paths
    watched_paths: Vec<PathBuf>,
}

impl FileWatcher {
    /// Create new file watcher
    pub fn new(debounce_ms: u64) -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let debounce_map = Arc::new(Mutex::new(HashMap::new()));
        let debounce_map_clone = debounce_map.clone();
        let debounce_duration = Duration::from_millis(debounce_ms);
        
        // Create notify watcher
        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let now = Instant::now();
                
                // Convert notify event to our event type
                let watch_events = match event.kind {
                    EventKind::Create(_) => {
                        event.paths.into_iter().map(|path| WatchEvent {
                            path,
                            event_type: WatchEventType::Created,
                            timestamp: now,
                        }).collect::<Vec<_>>()
                    }
                    EventKind::Modify(_) => {
                        event.paths.into_iter().map(|path| WatchEvent {
                            path,
                            event_type: WatchEventType::Modified,
                            timestamp: now,
                        }).collect::<Vec<_>>()
                    }
                    EventKind::Remove(_) => {
                        event.paths.into_iter().map(|path| WatchEvent {
                            path,
                            event_type: WatchEventType::Deleted,
                            timestamp: now,
                        }).collect::<Vec<_>>()
                    }
                    _ => vec![],
                };
                
                // Apply debouncing
                if let Ok(mut debounce) = debounce_map_clone.lock() {
                    for watch_event in watch_events {
                        if let Some(last_time) = debounce.get(&watch_event.path) {
                            if now.duration_since(*last_time) < debounce_duration {
                                continue; // Skip this event
                            }
                        }
                        
                        debounce.insert(watch_event.path.clone(), now);
                        let _ = tx.send(watch_event);
                    }
                }
            }
        })?;
        
        Ok(Self {
            watcher,
            rx,
            debounce_map,
            debounce_duration,
            watched_paths: Vec::new(),
        })
    }
    
    /// Watch a directory recursively
    pub fn watch_dir(&mut self, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.as_ref().to_path_buf();
        self.watcher.watch(&path, RecursiveMode::Recursive)?;
        self.watched_paths.push(path);
        Ok(())
    }
    
    /// Watch a specific file
    pub fn watch_file(&mut self, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.as_ref().to_path_buf();
        self.watcher.watch(&path, RecursiveMode::NonRecursive)?;
        self.watched_paths.push(path);
        Ok(())
    }
    
    /// Stop watching a path
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.as_ref();
        self.watcher.unwatch(path)?;
        self.watched_paths.retain(|p| p != path);
        Ok(())
    }
    
    /// Poll for events (non-blocking)
    pub fn poll_events(&mut self) -> Vec<WatchEvent> {
        let mut events = Vec::new();
        
        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
        }
        
        events
    }
    
    /// Wait for next event (blocking)
    pub async fn next_event(&mut self) -> Option<WatchEvent> {
        self.rx.recv().await
    }
    
    /// Get watched paths
    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched_paths
    }
    
    /// Clear debounce cache
    pub fn clear_debounce(&self) -> HotReloadResult<()> {
        self.debounce_map.lock().hot_reload_context("debounce_map")?.clear();
        Ok(())
    }
}

/// File filter for specific extensions
pub struct FileFilter {
    extensions: Vec<String>,
}

impl FileFilter {
    /// Create new filter with extensions
    pub fn new(extensions: Vec<&str>) -> Self {
        Self {
            extensions: extensions.iter().map(|s| s.to_string()).collect(),
        }
    }
    
    /// Check if file passes filter
    pub fn matches(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return self.extensions.iter().any(|e| e == ext_str);
            }
        }
        false
    }
}

/// Batch event processor
pub struct EventBatcher {
    events: Vec<WatchEvent>,
    batch_duration: Duration,
    last_batch: Instant,
}

impl EventBatcher {
    /// Create new event batcher
    pub fn new(batch_duration_ms: u64) -> Self {
        Self {
            events: Vec::new(),
            batch_duration: Duration::from_millis(batch_duration_ms),
            last_batch: Instant::now(),
        }
    }
    
    /// Add event to batch
    pub fn add_event(&mut self, event: WatchEvent) {
        self.events.push(event);
    }
    
    /// Get batched events if ready
    pub fn get_batch(&mut self) -> Option<Vec<WatchEvent>> {
        let now = Instant::now();
        
        if !self.events.is_empty() && now.duration_since(self.last_batch) >= self.batch_duration {
            let batch = std::mem::take(&mut self.events);
            self.last_batch = now;
            Some(batch)
        } else {
            None
        }
    }
    
    /// Force get current batch
    pub fn force_batch(&mut self) -> Vec<WatchEvent> {
        self.last_batch = Instant::now();
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    
    #[tokio::test]
    async fn test_file_watcher() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory for file watcher test");
        let mut watcher = FileWatcher::new(50).expect("Failed to create file watcher in test");
        
        // Watch temp directory
        watcher.watch_dir(temp_dir.path()).expect("Failed to watch temp directory in test");
        
        // Create a file
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "hello").expect("Failed to write test file in watcher test");
        
        // Wait for event
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let events = watcher.poll_events();
        assert!(!events.is_empty());
        
        let event = &events[0];
        assert_eq!(event.path, test_file);
        assert_eq!(event.event_type, WatchEventType::Created);
    }
    
    #[test]
    fn test_file_filter() {
        let filter = FileFilter::new(vec!["rs", "wgsl"]);
        
        assert!(filter.matches(Path::new("test.rs")));
        assert!(filter.matches(Path::new("shader.wgsl")));
        assert!(!filter.matches(Path::new("test.txt")));
        assert!(!filter.matches(Path::new("no_extension")));
    }
}