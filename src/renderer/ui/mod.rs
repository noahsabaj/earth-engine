use glam::Vec2;

/// UI Color representation
#[derive(Debug, Clone, Copy)]
pub struct UIColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl UIColor {
    pub const WHITE: UIColor = UIColor {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: UIColor = UIColor {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const RED: UIColor = UIColor {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: UIColor = UIColor {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: UIColor = UIColor {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

/// UI Rectangle
#[derive(Debug, Clone, Copy)]
pub struct UIRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl UIRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

/// UI Element type
#[derive(Debug, Clone)]
pub enum UIElement {
    Rect {
        rect: UIRect,
        color: UIColor,
        filled: bool,
        border_width: f32,
    },
    Text {
        text: String,
        position: Vec2,
        size: f32,
        color: UIColor,
    },
}

/// UI Renderer for immediate mode UI
pub struct UIRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    elements: Vec<UIElement>,
    screen_size: Vec2,
}

impl UIRenderer {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, width: f32, height: f32) -> Self {
        Self {
            device,
            queue,
            elements: Vec::new(),
            screen_size: Vec2::new(width, height),
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.screen_size = Vec2::new(width, height);
    }

    pub fn begin_frame(&mut self) {
        self.elements.clear();
    }

    pub fn draw_rect(&mut self, rect: UIRect, color: UIColor) {
        self.elements.push(UIElement::Rect {
            rect,
            color,
            filled: true,
            border_width: 0.0,
        });
    }

    pub fn draw_rect_outline(&mut self, rect: UIRect, color: UIColor, border_width: f32) {
        self.elements.push(UIElement::Rect {
            rect,
            color,
            filled: false,
            border_width,
        });
    }

    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: UIColor) {
        self.elements.push(UIElement::Text {
            text: text.to_string(),
            position: Vec2::new(x, y),
            size,
            color,
        });
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        // TODO: Implement actual rendering
        // For now, this is a placeholder
        // In a real implementation, this would:
        // 1. Create vertex buffers for all UI elements
        // 2. Set up a 2D orthographic projection
        // 3. Render all elements to the screen
    }
}
