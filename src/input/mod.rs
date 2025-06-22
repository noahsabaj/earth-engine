use std::collections::HashSet;
use winit::event::{ElementState, MouseButton};
pub use winit::keyboard::KeyCode;

#[derive(Debug)]
pub struct InputState {
    keys_pressed: HashSet<KeyCode>,
    mouse_buttons_pressed: HashSet<MouseButton>,
    mouse_delta: (f32, f32),
    cursor_locked: bool,
    last_mouse_pos: Option<(f32, f32)>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_pressed: HashSet::new(),
            mouse_buttons_pressed: HashSet::new(),
            mouse_delta: (0.0, 0.0),
            cursor_locked: false,
            last_mouse_pos: None,
        }
    }

    pub fn process_key(&mut self, key: KeyCode, state: ElementState) {
        match state {
            ElementState::Pressed => {
                self.keys_pressed.insert(key);
            }
            ElementState::Released => {
                self.keys_pressed.remove(&key);
            }
        }
    }

    pub fn process_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        match state {
            ElementState::Pressed => {
                self.mouse_buttons_pressed.insert(button);
            }
            ElementState::Released => {
                self.mouse_buttons_pressed.remove(&button);
            }
        }
    }

    pub fn process_mouse_motion(&mut self, delta: (f64, f64)) {
        // Check if this looks like absolute coordinates (WSL/X11 issue)
        if delta.0.abs() > 100.0 || delta.1.abs() > 100.0 {
            // This is likely absolute position, not delta
            let current_pos = (delta.0 as f32, delta.1 as f32);
            if let Some(last_pos) = self.last_mouse_pos {
                // Calculate actual delta from position difference
                let real_delta_x = current_pos.0 - last_pos.0;
                let real_delta_y = current_pos.1 - last_pos.1;

                // Only accumulate reasonable deltas
                if real_delta_x.abs() < 100.0 && real_delta_y.abs() < 100.0 {
                    self.mouse_delta.0 += real_delta_x;
                    self.mouse_delta.1 += real_delta_y;
                }
            }
            self.last_mouse_pos = Some(current_pos);
        } else {
            // Normal relative mouse motion
            self.mouse_delta.0 += delta.0 as f32;
            self.mouse_delta.1 += delta.1 as f32;
        }
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed.contains(&button)
    }

    pub fn get_mouse_delta(&self) -> (f32, f32) {
        self.mouse_delta
    }

    pub fn clear_mouse_delta(&mut self) {
        self.mouse_delta = (0.0, 0.0);
    }

    pub fn reset_mouse_tracking(&mut self) {
        self.last_mouse_pos = None;
        self.mouse_delta = (0.0, 0.0);
    }

    pub fn set_cursor_locked(&mut self, locked: bool) {
        self.cursor_locked = locked;
    }

    pub fn is_cursor_locked(&self) -> bool {
        self.cursor_locked
    }
}
