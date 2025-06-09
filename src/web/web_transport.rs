use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebTransport, WebTransportBidirectionalStream, WritableStream, ReadableStream};
use futures::stream::StreamExt;
use crate::web::WebError;
use std::sync::Arc;
use parking_lot::Mutex;

/// Message types for buffer streaming
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum MessageType {
    ChunkData = 0,
    PlayerUpdate = 1,
    BlockUpdate = 2,
    LightUpdate = 3,
    BatchUpdate = 4,
}

/// Buffer message header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct MessageHeader {
    msg_type: u8,
    flags: u8,
    sequence: u16,
    chunk_x: i32,
    chunk_y: i32,
    chunk_z: i32,
    data_size: u32,
}

/// WebTransport client for low-latency networking
pub struct WebTransportClient {
    /// WebTransport instance
    transport: Option<WebTransport>,
    
    /// Server URL
    server_url: String,
    
    /// Active streams
    streams: Arc<Mutex<Vec<WebTransportBidirectionalStream>>>,
    
    /// Connection state
    connected: Arc<Mutex<bool>>,
    
    /// Message handlers
    handlers: Arc<Mutex<MessageHandlers>>,
    
    /// Performance metrics
    metrics: Arc<Mutex<TransportMetrics>>,
}

/// Message handlers for different types
struct MessageHandlers {
    chunk_handler: Option<Box<dyn Fn(&[u8]) + Send + Sync>>,
    player_handler: Option<Box<dyn Fn(&[u8]) + Send + Sync>>,
    block_handler: Option<Box<dyn Fn(&[u8]) + Send + Sync>>,
}

/// Transport performance metrics
#[derive(Default)]
struct TransportMetrics {
    bytes_sent: u64,
    bytes_received: u64,
    messages_sent: u64,
    messages_received: u64,
    latency_ms: f64,
}

impl WebTransportClient {
    /// Create a new WebTransport client
    pub fn new(server_url: String) -> Self {
        log::info!("Creating WebTransportClient for {}", server_url);
        
        Self {
            transport: None,
            server_url,
            streams: Arc::new(Mutex::new(Vec::new())),
            connected: Arc::new(Mutex::new(false)),
            handlers: Arc::new(Mutex::new(MessageHandlers {
                chunk_handler: None,
                player_handler: None,
                block_handler: None,
            })),
            metrics: Arc::new(Mutex::new(TransportMetrics::default())),
        }
    }
    
    /// Connect to the server
    pub async fn connect(&mut self) -> Result<(), WebError> {
        log::info!("Connecting to WebTransport server: {}", self.server_url);
        
        // Create WebTransport instance
        let transport = WebTransport::new(&self.server_url)
            .map_err(|e| WebError::TransportError(format!("Failed to create transport: {:?}", e)))?;
        
        // Wait for connection
        let ready_promise = transport.ready();
        wasm_bindgen_futures::JsFuture::from(ready_promise).await
            .map_err(|e| WebError::TransportError(format!("Connection failed: {:?}", e)))?;
        
        *self.connected.lock() = true;
        self.transport = Some(transport);
        
        // Start receiving messages
        self.start_receive_loop();
        
        log::info!("WebTransport connected successfully");
        Ok(())
    }
    
    /// Disconnect from the server
    pub fn disconnect(&mut self) {
        if let Some(transport) = &self.transport {
            transport.close();
            *self.connected.lock() = false;
            self.transport = None;
        }
    }
    
    /// Send raw buffer data
    pub async fn send_buffer(
        &self,
        msg_type: MessageType,
        chunk_pos: (i32, i32, i32),
        data: &[u8],
    ) -> Result<(), WebError> {
        if !*self.connected.lock() {
            return Err(WebError::TransportError("Not connected".into()));
        }
        
        let transport = self.transport.as_ref()
            .ok_or(WebError::TransportError("No transport".into()))?;
        
        // Create header
        let header = MessageHeader {
            msg_type: msg_type as u8,
            flags: 0,
            sequence: 0, // TODO: Add sequencing
            chunk_x: chunk_pos.0,
            chunk_y: chunk_pos.1,
            chunk_z: chunk_pos.2,
            data_size: data.len() as u32,
        };
        
        // Get or create stream
        let stream = self.get_or_create_stream().await?;
        let writable = stream.writable();
        
        // Write header + data
        let mut buffer = Vec::with_capacity(std::mem::size_of::<MessageHeader>() + data.len());
        buffer.extend_from_slice(&header_to_bytes(&header));
        buffer.extend_from_slice(data);
        
        // Send data
        write_to_stream(&writable, &buffer).await?;
        
        // Update metrics
        let mut metrics = self.metrics.lock();
        metrics.bytes_sent += buffer.len() as u64;
        metrics.messages_sent += 1;
        
        Ok(())
    }
    
    /// Send chunk data
    pub async fn send_chunk_data(
        &self,
        chunk_pos: (i32, i32, i32),
        voxel_data: &[u32],
    ) -> Result<(), WebError> {
        // Convert voxel data to bytes
        let bytes: Vec<u8> = voxel_data.iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        
        self.send_buffer(MessageType::ChunkData, chunk_pos, &bytes).await
    }
    
    /// Request chunk from server
    pub async fn request_chunk(&self, chunk_pos: (i32, i32, i32)) -> Result<(), WebError> {
        // Send empty chunk request
        self.send_buffer(MessageType::ChunkData, chunk_pos, &[]).await
    }
    
    /// Set chunk data handler
    pub fn set_chunk_handler<F>(&self, handler: F)
    where
        F: Fn(&[u8]) + Send + Sync + 'static,
    {
        self.handlers.lock().chunk_handler = Some(Box::new(handler));
    }
    
    /// Set player update handler
    pub fn set_player_handler<F>(&self, handler: F)
    where
        F: Fn(&[u8]) + Send + Sync + 'static,
    {
        self.handlers.lock().player_handler = Some(Box::new(handler));
    }
    
    /// Set block update handler
    pub fn set_block_handler<F>(&self, handler: F)
    where
        F: Fn(&[u8]) + Send + Sync + 'static,
    {
        self.handlers.lock().block_handler = Some(Box::new(handler));
    }
    
    /// Get transport metrics
    pub fn get_metrics(&self) -> TransportMetrics {
        self.metrics.lock().clone()
    }
    
    /// Start receive loop
    fn start_receive_loop(&self) {
        let transport = match &self.transport {
            Some(t) => t.clone(),
            None => return,
        };
        
        let streams = self.streams.clone();
        let handlers = self.handlers.clone();
        let metrics = self.metrics.clone();
        let connected = self.connected.clone();
        
        wasm_bindgen_futures::spawn_local(async move {
            // Accept incoming streams
            let incoming = transport.incoming_bidirectional_streams();
            
            loop {
                // Check if still connected
                if !*connected.lock() {
                    break;
                }
                
                // Accept next stream
                match accept_stream(&incoming).await {
                    Ok(stream) => {
                        // Store stream
                        streams.lock().push(stream.clone());
                        
                        // Handle messages on this stream
                        handle_stream_messages(stream, handlers.clone(), metrics.clone()).await;
                    }
                    Err(e) => {
                        log::error!("Failed to accept stream: {:?}", e);
                        break;
                    }
                }
            }
        });
    }
    
    /// Get or create a stream for sending
    async fn get_or_create_stream(&self) -> Result<WebTransportBidirectionalStream, WebError> {
        // Try to reuse existing stream
        if let Some(stream) = self.streams.lock().first() {
            return Ok(stream.clone());
        }
        
        // Create new stream
        let transport = self.transport.as_ref()
            .ok_or(WebError::TransportError("No transport".into()))?;
        
        let stream_promise = transport.create_bidirectional_stream();
        let stream = wasm_bindgen_futures::JsFuture::from(stream_promise).await
            .map_err(|e| WebError::TransportError(format!("Failed to create stream: {:?}", e)))?
            .dyn_into::<WebTransportBidirectionalStream>()
            .map_err(|_| WebError::TransportError("Invalid stream type".into()))?;
        
        self.streams.lock().push(stream.clone());
        Ok(stream)
    }
}

/// Handle messages on a stream
async fn handle_stream_messages(
    stream: WebTransportBidirectionalStream,
    handlers: Arc<Mutex<MessageHandlers>>,
    metrics: Arc<Mutex<TransportMetrics>>,
) {
    let readable = stream.readable();
    
    loop {
        match read_from_stream(&readable).await {
            Ok(data) => {
                // Parse header
                if data.len() < std::mem::size_of::<MessageHeader>() {
                    log::warn!("Received invalid message (too small)");
                    continue;
                }
                
                let header = parse_header(&data[..std::mem::size_of::<MessageHeader>()]);
                let payload = &data[std::mem::size_of::<MessageHeader>()..];
                
                // Update metrics
                {
                    let mut m = metrics.lock();
                    m.bytes_received += data.len() as u64;
                    m.messages_received += 1;
                }
                
                // Dispatch to handler
                let handlers = handlers.lock();
                match header.msg_type {
                    0 => { // ChunkData
                        if let Some(handler) = &handlers.chunk_handler {
                            handler(payload);
                        }
                    }
                    1 => { // PlayerUpdate
                        if let Some(handler) = &handlers.player_handler {
                            handler(payload);
                        }
                    }
                    2 => { // BlockUpdate
                        if let Some(handler) = &handlers.block_handler {
                            handler(payload);
                        }
                    }
                    _ => {
                        log::warn!("Unknown message type: {}", header.msg_type);
                    }
                }
            }
            Err(e) => {
                log::error!("Stream read error: {:?}", e);
                break;
            }
        }
    }
}

/// Convert header to bytes
fn header_to_bytes(header: &MessageHeader) -> [u8; std::mem::size_of::<MessageHeader>()] {
    unsafe { std::mem::transmute_copy(header) }
}

/// Parse header from bytes
fn parse_header(bytes: &[u8]) -> MessageHeader {
    unsafe {
        std::ptr::read(bytes.as_ptr() as *const MessageHeader)
    }
}

/// Write data to a WritableStream
async fn write_to_stream(stream: &WritableStream, data: &[u8]) -> Result<(), WebError> {
    use js_sys::Uint8Array;
    
    let array = Uint8Array::new_with_length(data.len() as u32);
    array.copy_from(data);
    
    let writer = stream.get_writer()
        .map_err(|e| WebError::TransportError(format!("Failed to get writer: {:?}", e)))?;
    
    let write_promise = writer.write_with_chunk(&array);
    wasm_bindgen_futures::JsFuture::from(write_promise).await
        .map_err(|e| WebError::TransportError(format!("Write failed: {:?}", e)))?;
    
    writer.release_lock();
    Ok(())
}

/// Read data from a ReadableStream
async fn read_from_stream(stream: &ReadableStream) -> Result<Vec<u8>, WebError> {
    use js_sys::Uint8Array;
    
    let reader = stream.get_reader()
        .dyn_into::<web_sys::ReadableStreamDefaultReader>()
        .map_err(|_| WebError::TransportError("Failed to get reader".into()))?;
    
    let result = wasm_bindgen_futures::JsFuture::from(reader.read()).await
        .map_err(|e| WebError::TransportError(format!("Read failed: {:?}", e)))?;
    
    let done = js_sys::Reflect::get(&result, &JsValue::from_str("done"))
        .map_err(|_| WebError::TransportError("Failed to get done".into()))?
        .as_bool()
        .unwrap_or(true);
    
    if done {
        return Err(WebError::TransportError("Stream closed".into()));
    }
    
    let value = js_sys::Reflect::get(&result, &JsValue::from_str("value"))
        .map_err(|_| WebError::TransportError("Failed to get value".into()))?;
    
    let array = value.dyn_into::<Uint8Array>()
        .map_err(|_| WebError::TransportError("Invalid data type".into()))?;
    
    let mut data = vec![0u8; array.length() as usize];
    array.copy_to(&mut data);
    
    reader.release_lock();
    Ok(data)
}

/// Accept a stream from incoming streams
async fn accept_stream(
    incoming: &web_sys::ReadableStream,
) -> Result<WebTransportBidirectionalStream, WebError> {
    let reader = incoming.get_reader()
        .dyn_into::<web_sys::ReadableStreamDefaultReader>()
        .map_err(|_| WebError::TransportError("Failed to get reader".into()))?;
    
    let result = wasm_bindgen_futures::JsFuture::from(reader.read()).await
        .map_err(|e| WebError::TransportError(format!("Read failed: {:?}", e)))?;
    
    let value = js_sys::Reflect::get(&result, &JsValue::from_str("value"))
        .map_err(|_| WebError::TransportError("Failed to get value".into()))?;
    
    let stream = value.dyn_into::<WebTransportBidirectionalStream>()
        .map_err(|_| WebError::TransportError("Invalid stream type".into()))?;
    
    reader.release_lock();
    Ok(stream)
}

impl Clone for TransportMetrics {
    fn clone(&self) -> Self {
        *self
    }
}