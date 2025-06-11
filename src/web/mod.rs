/// WebGPU Buffer-First Architecture
/// 
/// This module implements a pure data-oriented version of Earth Engine for web browsers.
/// It uses WebGPU for all rendering and computation, with zero-copy operations throughout.
/// 
/// Key principles:
/// - All data lives in GPU buffers
/// - No object-oriented abstractions
/// - Direct manipulation of typed arrays
/// - Leverages browser's unified memory architecture
/// - Buffer-based networking with WebTransport

#[cfg(target_arch = "wasm32")]
pub mod webgpu_context;
#[cfg(target_arch = "wasm32")]
pub mod web_world_buffer;
#[cfg(target_arch = "wasm32")]
pub mod web_renderer;
#[cfg(target_arch = "wasm32")]
pub mod buffer_manager;
// WebTransport not yet available in web-sys
// #[cfg(target_arch = "wasm32")]
// pub mod web_transport;
#[cfg(target_arch = "wasm32")]
pub mod asset_streaming;

#[cfg(target_arch = "wasm32")]
pub use webgpu_context::{WebGpuContext, WebGpuConfig};
#[cfg(target_arch = "wasm32")]
pub use web_world_buffer::WebWorldBuffer;
#[cfg(target_arch = "wasm32")]
pub use web_renderer::WebRenderer;
#[cfg(target_arch = "wasm32")]
pub use buffer_manager::{BufferManager, BufferHandle};
// #[cfg(target_arch = "wasm32")]
// pub use web_transport::WebTransportClient;
#[cfg(target_arch = "wasm32")]
pub use asset_streaming::AssetStreamer;

/// Web-specific error type
#[derive(Debug, thiserror::Error)]
pub enum WebError {
    #[error("WebGPU not supported in this browser")]
    WebGpuNotSupported,
    
    #[error("Failed to get WebGPU adapter")]
    AdapterError,
    
    #[error("Failed to get WebGPU device")]
    DeviceError,
    
    #[error("Buffer operation failed: {0}")]
    BufferError(String),
    
    #[error("JavaScript error: {0}")]
    JsError(String),
    
    // #[error("WebTransport error: {0}")]
    // TransportError(String),
}

/// Entry point for web builds
#[cfg(target_arch = "wasm32")]
pub async fn run_web() -> Result<(), WebError> {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();
    
    // Initialize logging
    console_log::init_with_level(log::Level::Info)
        .expect("Failed to initialize console logging");
    
    log::info!("Earth Engine Web - Pure Data-Oriented Architecture");
    
    // Get canvas element
    let window = web_sys::window().ok_or(WebError::JsError("No window".into()))?;
    let document = window.document().ok_or(WebError::JsError("No document".into()))?;
    let canvas = document
        .get_element_by_id("earth-engine-canvas")
        .ok_or(WebError::JsError("Canvas not found".into()))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| WebError::JsError("Not a canvas element".into()))?;
    
    // Initialize WebGPU
    let context = WebGpuContext::new(&canvas).await?;
    
    // Create web world buffer (uses GPU buffers from Sprint 21)
    let world_buffer = WebWorldBuffer::new(&context)?;
    
    // Create renderer
    let renderer = WebRenderer::new(&context, &world_buffer)?;
    
    // Start render loop
    start_render_loop(context, world_buffer, renderer);
    
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn start_render_loop(
    context: WebGpuContext,
    world_buffer: WebWorldBuffer,
    renderer: WebRenderer,
) {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    
    let f = std::rc::Rc::new(std::cell::RefCell::new(None));
    let g = f.clone();
    
    *g.borrow_mut() = Some(wasm_bindgen::closure::Closure::wrap(Box::new(move || {
        // Render frame
        renderer.render(&context, &world_buffer);
        
        // Request next frame
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));
    
    request_animation_frame(g.borrow().as_ref().unwrap());
}

#[cfg(target_arch = "wasm32")]
fn request_animation_frame(f: &wasm_bindgen::closure::Closure<dyn FnMut()>) {
    web_sys::window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .unwrap();
}