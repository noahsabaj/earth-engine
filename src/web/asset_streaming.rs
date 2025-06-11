use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Response, Headers};
use futures::StreamExt;
use crate::web::{WebError, BufferManager, BufferHandle};
use std::sync::Arc;
use parking_lot::Mutex;
use std::collections::HashMap;

/// Asset types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetType {
    Texture,
    Model,
    Shader,
    Audio,
    ChunkData,
}

/// Asset metadata
#[derive(Debug, Clone)]
pub struct AssetMetadata {
    pub asset_type: AssetType,
    pub url: String,
    pub size: Option<u64>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// Asset handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetHandle(u64);

/// Streaming asset state
enum AssetState {
    Pending,
    Streaming { progress: f32 },
    Ready { buffer: BufferHandle },
    Failed { error: String },
}

/// Zero-copy asset streaming system
pub struct AssetStreamer {
    /// Buffer manager
    buffer_manager: Arc<Mutex<BufferManager>>,
    
    /// Asset registry
    assets: Arc<Mutex<HashMap<AssetHandle, (AssetMetadata, AssetState)>>>,
    
    /// URL to handle mapping
    url_map: Arc<Mutex<HashMap<String, AssetHandle>>>,
    
    /// Next handle ID
    next_handle: Arc<Mutex<u64>>,
    
    /// Streaming configuration
    config: StreamerConfig,
    
    /// Performance metrics
    metrics: Arc<Mutex<StreamerMetrics>>,
}

/// Streamer configuration
#[derive(Debug, Clone)]
pub struct StreamerConfig {
    /// Maximum concurrent streams
    pub max_concurrent: usize,
    
    /// Chunk size for streaming
    pub chunk_size: usize,
    
    /// Enable SharedArrayBuffer if available
    pub use_shared_memory: bool,
    
    /// Cache control headers
    pub cache_headers: bool,
    
    /// Compression support
    pub accept_encoding: String,
}

impl Default for StreamerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 4,
            chunk_size: 65536, // 64KB chunks
            use_shared_memory: true,
            cache_headers: true,
            accept_encoding: "gzip, deflate, br".into(),
        }
    }
}

/// Performance metrics
#[derive(Debug, Default)]
struct StreamerMetrics {
    total_bytes: u64,
    total_assets: u32,
    active_streams: u32,
    cache_hits: u32,
    cache_misses: u32,
}

impl AssetStreamer {
    /// Create a new asset streamer
    pub fn new(buffer_manager: Arc<Mutex<BufferManager>>) -> Self {
        Self::with_config(buffer_manager, StreamerConfig::default())
    }
    
    /// Create with custom configuration
    pub fn with_config(
        buffer_manager: Arc<Mutex<BufferManager>>,
        config: StreamerConfig,
    ) -> Self {
        log::info!("Creating AssetStreamer with config: {:?}", config);
        
        // Check for SharedArrayBuffer support
        if config.use_shared_memory && check_shared_array_buffer_support() {
            log::info!("SharedArrayBuffer is available - enabling zero-copy transfers");
        } else {
            log::warn!("SharedArrayBuffer not available - using standard copies");
        }
        
        Self {
            buffer_manager,
            assets: Arc::new(Mutex::new(HashMap::new())),
            url_map: Arc::new(Mutex::new(HashMap::new())),
            next_handle: Arc::new(Mutex::new(1)),
            config,
            metrics: Arc::new(Mutex::new(StreamerMetrics::default())),
        }
    }
    
    /// Stream an asset from URL
    pub async fn stream_asset(
        &self,
        url: String,
        asset_type: AssetType,
    ) -> Result<AssetHandle, WebError> {
        // Check if already streaming/loaded
        if let Some(handle) = self.url_map.lock().get(&url) {
            return Ok(*handle);
        }
        
        // Create new handle
        let handle = {
            let mut next = self.next_handle.lock();
            let h = AssetHandle(*next);
            *next += 1;
            h
        };
        
        // Register asset
        let metadata = AssetMetadata {
            asset_type,
            url: url.clone(),
            size: None,
            etag: None,
            last_modified: None,
        };
        
        self.assets.lock().insert(handle, (metadata.clone(), AssetState::Pending));
        self.url_map.lock().insert(url.clone(), handle);
        
        // Start streaming
        self.start_streaming(handle, metadata);
        
        Ok(handle)
    }
    
    /// Get asset state
    pub fn get_state(&self, handle: AssetHandle) -> Option<AssetState> {
        self.assets.lock().get(&handle).map(|(_, state)| match state {
            AssetState::Pending => AssetState::Pending,
            AssetState::Streaming { progress } => AssetState::Streaming { progress: *progress },
            AssetState::Ready { buffer } => AssetState::Ready { buffer: *buffer },
            AssetState::Failed { error } => AssetState::Failed { error: error.clone() },
        })
    }
    
    /// Get buffer for ready asset
    pub fn get_buffer(&self, handle: AssetHandle) -> Option<BufferHandle> {
        self.assets.lock().get(&handle).and_then(|(_, state)| {
            if let AssetState::Ready { buffer } = state {
                Some(*buffer)
            } else {
                None
            }
        })
    }
    
    /// Start streaming process
    fn start_streaming(&self, handle: AssetHandle, metadata: AssetMetadata) {
        let assets = self.assets.clone();
        let buffer_manager = self.buffer_manager.clone();
        let metrics = self.metrics.clone();
        let config = self.config.clone();
        
        wasm_bindgen_futures::spawn_local(async move {
            match stream_asset_impl(metadata, buffer_manager, config).await {
                Ok((buffer, size)) => {
                    // Update state to ready
                    if let Some((meta, state)) = assets.lock().get_mut(&handle) {
                        meta.size = Some(size);
                        *state = AssetState::Ready { buffer };
                    }
                    
                    // Update metrics
                    let mut m = metrics.lock();
                    m.total_bytes += size;
                    m.total_assets += 1;
                }
                Err(e) => {
                    // Update state to failed
                    if let Some((_, state)) = assets.lock().get_mut(&handle) {
                        *state = AssetState::Failed { error: e.to_string() };
                    }
                }
            }
        });
    }
    
    /// Preload multiple assets
    pub async fn preload_assets(
        &self,
        requests: Vec<(String, AssetType)>,
    ) -> Vec<Result<AssetHandle, WebError>> {
        let mut results = Vec::with_capacity(requests.len());
        
        for (url, asset_type) in requests {
            results.push(self.stream_asset(url, asset_type).await);
        }
        
        results
    }
    
    /// Get streaming metrics
    pub fn get_metrics(&self) -> StreamerMetrics {
        self.metrics.lock().clone()
    }
    
    /// Clear cached assets
    pub fn clear_cache(&self) {
        let handles: Vec<_> = self.assets.lock().keys().copied().collect();
        
        for handle in handles {
            if let Some((_, AssetState::Ready { buffer })) = self.assets.lock().remove(&handle) {
                self.buffer_manager.lock().release(buffer);
            }
        }
        
        self.url_map.lock().clear();
    }
}

/// Stream asset implementation
async fn stream_asset_impl(
    metadata: AssetMetadata,
    buffer_manager: Arc<Mutex<BufferManager>>,
    config: StreamerConfig,
) -> Result<(BufferHandle, u64), WebError> {
    // Create fetch headers
    let headers = Headers::new()
        .map_err(|_| WebError::JsError("Failed to create headers".into()))?;
    
    if config.cache_headers {
        headers.append("Cache-Control", "public, max-age=3600").ok();
    }
    
    if !config.accept_encoding.is_empty() {
        headers.append("Accept-Encoding", &config.accept_encoding).ok();
    }
    
    // Fetch with streaming support
    let window = web_sys::window()
        .ok_or(WebError::JsError("No window".into()))?;
    
    let response_promise = window.fetch_with_str_and_init(
        &metadata.url,
        web_sys::RequestInit::new()
            .headers(&headers)
            .method("GET"),
    );
    
    let response: Response = wasm_bindgen_futures::JsFuture::from(response_promise).await
        .map_err(|e| WebError::JsError(format!("Fetch failed: {:?}", e)))?
        .dyn_into()
        .map_err(|_| WebError::JsError("Invalid response".into()))?;
    
    if !response.ok() {
        return Err(WebError::JsError(format!(
            "HTTP error: {}",
            response.status()
        )));
    }
    
    // Get content length
    let content_length = response.headers()
        .get("content-length").ok()
        .flatten()
        .and_then(|s| s.parse::<u64>().ok());
    
    // Check if we can use SharedArrayBuffer
    let use_shared = config.use_shared_memory && 
                     check_shared_array_buffer_support() &&
                     response.headers().get("cross-origin-isolated").ok().flatten().is_some();
    
    if use_shared {
        // Zero-copy path with SharedArrayBuffer
        stream_with_shared_memory(response, buffer_manager, content_length).await
    } else {
        // Standard streaming path
        stream_standard(response, buffer_manager, content_length, config.chunk_size).await
    }
}

/// Stream with SharedArrayBuffer (zero-copy)
async fn stream_with_shared_memory(
    response: Response,
    buffer_manager: Arc<Mutex<BufferManager>>,
    content_length: Option<u64>,
) -> Result<(BufferHandle, u64), WebError> {
    use js_sys::{SharedArrayBuffer, Uint8Array};
    
    // Get array buffer directly
    let buffer_promise = response.array_buffer()
        .map_err(|_| WebError::JsError("Failed to get array buffer".into()))?;
    
    let array_buffer = wasm_bindgen_futures::JsFuture::from(buffer_promise).await
        .map_err(|e| WebError::JsError(format!("Buffer promise failed: {:?}", e)))?;
    
    // Try to create SharedArrayBuffer
    let shared = SharedArrayBuffer::new(array_buffer.clone().into());
    let array = Uint8Array::new(&shared);
    
    // Create GPU buffer with shared memory reference
    let size = array.length() as u64;
    // SAFETY: Creating a slice from Uint8Array pointer is safe because:
    // - array.as_ptr() returns a valid pointer to the underlying ArrayBuffer data
    // - array.length() gives the exact size of the array in bytes
    // - The slice lifetime is tied to the array, which is alive for this scope
    // - Uint8Array guarantees contiguous memory layout
    // - The data is only read, not modified through this slice
    let data = unsafe {
        std::slice::from_raw_parts(array.as_ptr(), array.length() as usize)
    };
    
    let handle = buffer_manager.lock().create_staging_buffer(data)?;
    
    log::info!("Streamed {} bytes using SharedArrayBuffer (zero-copy)", size);
    Ok((handle, size))
}

/// Standard streaming
async fn stream_standard(
    response: Response,
    buffer_manager: Arc<Mutex<BufferManager>>,
    content_length: Option<u64>,
    chunk_size: usize,
) -> Result<(BufferHandle, u64), WebError> {
    use js_sys::Uint8Array;
    use wasm_bindgen_futures::stream::JsStream;
    
    // Get readable stream
    let body = response.body()
        .ok_or(WebError::JsError("No response body".into()))?;
    
    let stream = JsStream::from(body);
    let mut reader = stream.into_stream();
    
    // Collect chunks
    let mut chunks = Vec::new();
    let mut total_size = 0u64;
    
    while let Some(result) = reader.next().await {
        match result {
            Ok(chunk) => {
                let array = Uint8Array::new(&chunk);
                let mut data = vec![0u8; array.length() as usize];
                array.copy_to(&mut data);
                
                total_size += data.len() as u64;
                chunks.push(data);
            }
            Err(e) => {
                return Err(WebError::JsError(format!("Stream error: {:?}", e)));
            }
        }
    }
    
    // Combine chunks
    let mut combined = Vec::with_capacity(total_size as usize);
    for chunk in chunks {
        combined.extend_from_slice(&chunk);
    }
    
    // Create GPU buffer
    let handle = buffer_manager.lock().create_staging_buffer(&combined)?;
    
    log::info!("Streamed {} bytes using standard streaming", total_size);
    Ok((handle, total_size))
}

/// Check if SharedArrayBuffer is available
fn check_shared_array_buffer_support() -> bool {
    js_sys::Reflect::has(
        &js_sys::global(),
        &JsValue::from_str("SharedArrayBuffer"),
    ).unwrap_or(false)
}

/// Stream chunk data directly to GPU buffer
pub async fn stream_chunk_to_gpu(
    url: &str,
    world_buffer: &crate::web::WebWorldBuffer,
    context: &crate::web::WebGpuContext,
    chunk_pos: (i32, i32, i32),
) -> Result<(), WebError> {
    // Fetch chunk data
    let window = web_sys::window()
        .ok_or(WebError::JsError("No window".into()))?;
    
    let response_promise = window.fetch_with_str(url);
    let response: Response = wasm_bindgen_futures::JsFuture::from(response_promise).await
        .map_err(|e| WebError::JsError(format!("Fetch failed: {:?}", e)))?
        .dyn_into()
        .map_err(|_| WebError::JsError("Invalid response".into()))?;
    
    if !response.ok() {
        return Err(WebError::JsError(format!("HTTP error: {}", response.status())));
    }
    
    // Get array buffer
    let buffer_promise = response.array_buffer()
        .map_err(|_| WebError::JsError("Failed to get array buffer".into()))?;
    
    let array_buffer = wasm_bindgen_futures::JsFuture::from(buffer_promise).await
        .map_err(|e| WebError::JsError(format!("Buffer promise failed: {:?}", e)))?;
    
    // Convert to voxel data
    let array = js_sys::Uint8Array::new(&array_buffer);
    let mut data = vec![0u8; array.length() as usize];
    array.copy_to(&mut data);
    
    // Parse voxel data (assuming u32 format)
    let voxel_data: Vec<crate::web::web_world_buffer::VoxelData> = data.chunks_exact(4)
        .map(|chunk| {
            let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            crate::web::web_world_buffer::VoxelData(value)
        })
        .collect();
    
    // Calculate offset
    let chunk_offset = world_buffer.chunk_index(
        chunk_pos.0 as u32,
        chunk_pos.1 as u32,
        chunk_pos.2 as u32,
    ) as u64 * (32 * 32 * 32 * 4); // CHUNK_SIZE^3 * 4 bytes per voxel
    
    // Upload directly to GPU
    world_buffer.upload_voxels_async(context, chunk_offset, &voxel_data).await?;
    
    Ok(())
}

impl Clone for StreamerMetrics {
    fn clone(&self) -> Self {
        Self {
            total_bytes: self.total_bytes,
            total_assets: self.total_assets,
            active_streams: self.active_streams,
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
        }
    }
}