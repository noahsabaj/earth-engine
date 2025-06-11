// Minimal WASM demo showing Earth Engine concepts
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

#[wasm_bindgen]
pub struct EarthEngineDemo {
    width: u32,
    height: u32,
}

#[wasm_bindgen]
impl EarthEngineDemo {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: &HtmlCanvasElement) -> Result<EarthEngineDemo, JsValue> {
        console_error_panic_hook::set_once();
        
        let width = canvas.width();
        let height = canvas.height();
        
        web_sys::console::log_1(&"Earth Engine WASM Demo initialized".into());
        
        Ok(EarthEngineDemo { width, height })
    }
    
    #[wasm_bindgen]
    pub fn render(&self, time: f32) {
        // This would render using WebGPU/WebGL
        web_sys::console::log_1(&format!("Rendering frame at time: {}", time).into());
    }
    
    #[wasm_bindgen]
    pub fn get_info(&self) -> String {
        format!("Earth Engine v0.35.0 - {}x{} - Data-Oriented Architecture", self.width, self.height)
    }
}