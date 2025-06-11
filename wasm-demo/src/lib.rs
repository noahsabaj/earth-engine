// Minimal WASM demo showing Earth Engine concepts
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

#[wasm_bindgen]
pub struct EarthEngineDemo {
    width: u32,
    height: u32,
    frame_count: u32,
}

#[wasm_bindgen]
impl EarthEngineDemo {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: &HtmlCanvasElement) -> Result<EarthEngineDemo, JsValue> {
        console_error_panic_hook::set_once();
        
        let width = canvas.width();
        let height = canvas.height();
        
        web_sys::console::log_1(&"Earth Engine WASM Demo initialized".into());
        web_sys::console::log_1(&format!("Canvas size: {}x{}", width, height).into());
        
        Ok(EarthEngineDemo { 
            width, 
            height,
            frame_count: 0,
        })
    }
    
    #[wasm_bindgen]
    pub fn render(&mut self, time: f32) {
        self.frame_count += 1;
        
        if self.frame_count % 60 == 0 {
            web_sys::console::log_1(&format!("Frame {}: time={:.2}", self.frame_count, time).into());
        }
    }
    
    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        web_sys::console::log_1(&format!("Resized to {}x{}", width, height).into());
    }
    
    #[wasm_bindgen]
    pub fn get_info(&self) -> String {
        format!(
            "Earth Engine v0.35.0\n\
            Resolution: {}x{}\n\
            Architecture: Data-Oriented\n\
            Frames: {}", 
            self.width, self.height, self.frame_count
        )
    }
}