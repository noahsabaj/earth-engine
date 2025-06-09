use crate::inventory::{PlayerInventory, HOTBAR_SIZE, INVENTORY_SIZE};
use crate::renderer::ui::{UIRenderer, UIElement, UIRect, UIColor};
use glam::Vec2;

/// Size of inventory slots in pixels
const SLOT_SIZE: f32 = 48.0;
const SLOT_PADDING: f32 = 4.0;
const INVENTORY_PADDING: f32 = 16.0;

/// State of the inventory UI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InventoryUIState {
    Closed,
    Open,
}

/// Inventory UI renderer
pub struct InventoryUI {
    state: InventoryUIState,
    selected_slot: Option<usize>,
    hotbar_position: Vec2,
    inventory_position: Vec2,
}

impl InventoryUI {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        // Center hotbar at bottom of screen
        let hotbar_width = HOTBAR_SIZE as f32 * (SLOT_SIZE + SLOT_PADDING);
        let hotbar_x = (screen_width - hotbar_width) / 2.0;
        let hotbar_y = screen_height - SLOT_SIZE - INVENTORY_PADDING * 2.0;
        
        // Center inventory in middle of screen
        let inventory_cols = 9;
        let inventory_rows = 4; // 1 row hotbar + 3 rows main inventory
        let inventory_width = inventory_cols as f32 * (SLOT_SIZE + SLOT_PADDING);
        let inventory_height = inventory_rows as f32 * (SLOT_SIZE + SLOT_PADDING);
        let inventory_x = (screen_width - inventory_width) / 2.0;
        let inventory_y = (screen_height - inventory_height) / 2.0;
        
        Self {
            state: InventoryUIState::Closed,
            selected_slot: None,
            hotbar_position: Vec2::new(hotbar_x, hotbar_y),
            inventory_position: Vec2::new(inventory_x, inventory_y),
        }
    }
    
    /// Update screen dimensions
    pub fn resize(&mut self, screen_width: f32, screen_height: f32) {
        // Recalculate positions
        let hotbar_width = HOTBAR_SIZE as f32 * (SLOT_SIZE + SLOT_PADDING);
        let hotbar_x = (screen_width - hotbar_width) / 2.0;
        let hotbar_y = screen_height - SLOT_SIZE - INVENTORY_PADDING * 2.0;
        self.hotbar_position = Vec2::new(hotbar_x, hotbar_y);
        
        let inventory_cols = 9;
        let inventory_rows = 4;
        let inventory_width = inventory_cols as f32 * (SLOT_SIZE + SLOT_PADDING);
        let inventory_height = inventory_rows as f32 * (SLOT_SIZE + SLOT_PADDING);
        let inventory_x = (screen_width - inventory_width) / 2.0;
        let inventory_y = (screen_height - inventory_height) / 2.0;
        self.inventory_position = Vec2::new(inventory_x, inventory_y);
    }
    
    /// Open the inventory
    pub fn open(&mut self) {
        self.state = InventoryUIState::Open;
    }
    
    /// Close the inventory
    pub fn close(&mut self) {
        self.state = InventoryUIState::Closed;
        self.selected_slot = None;
    }
    
    /// Toggle inventory open/closed
    pub fn toggle(&mut self) {
        match self.state {
            InventoryUIState::Open => self.close(),
            InventoryUIState::Closed => self.open(),
        }
    }
    
    /// Check if inventory is open
    pub fn is_open(&self) -> bool {
        self.state == InventoryUIState::Open
    }
    
    /// Handle mouse click
    pub fn handle_click(&mut self, mouse_x: f32, mouse_y: f32) -> Option<usize> {
        if self.state != InventoryUIState::Open {
            return None;
        }
        
        // Check inventory slots
        for i in 0..INVENTORY_SIZE {
            let (x, y) = self.get_slot_position(i, true);
            let rect = UIRect {
                x,
                y,
                width: SLOT_SIZE,
                height: SLOT_SIZE,
            };
            
            if rect.contains(mouse_x, mouse_y) {
                self.selected_slot = Some(i);
                return Some(i);
            }
        }
        
        None
    }
    
    /// Get the position of a slot
    fn get_slot_position(&self, index: usize, full_inventory: bool) -> (f32, f32) {
        if index < HOTBAR_SIZE {
            // Hotbar slot
            let x = self.hotbar_position.x + index as f32 * (SLOT_SIZE + SLOT_PADDING);
            let y = if full_inventory {
                // Show in full inventory view
                self.inventory_position.y + 3.0 * (SLOT_SIZE + SLOT_PADDING)
            } else {
                // Show at bottom of screen
                self.hotbar_position.y
            };
            (x, y)
        } else {
            // Main inventory slot
            let adjusted_index = index - HOTBAR_SIZE;
            let col = adjusted_index % 9;
            let row = adjusted_index / 9;
            let x = self.inventory_position.x + col as f32 * (SLOT_SIZE + SLOT_PADDING);
            let y = self.inventory_position.y + row as f32 * (SLOT_SIZE + SLOT_PADDING);
            (x, y)
        }
    }
    
    /// Render the inventory UI
    pub fn render(&self, ui_renderer: &mut UIRenderer, inventory: &PlayerInventory) {
        match self.state {
            InventoryUIState::Closed => {
                // Only render hotbar
                self.render_hotbar(ui_renderer, inventory);
            }
            InventoryUIState::Open => {
                // Render full inventory
                self.render_full_inventory(ui_renderer, inventory);
            }
        }
    }
    
    /// Render just the hotbar
    fn render_hotbar(&self, ui_renderer: &mut UIRenderer, inventory: &PlayerInventory) {
        let selected_index = inventory.selected_hotbar_index();
        
        for i in 0..HOTBAR_SIZE {
            let (x, y) = self.get_slot_position(i, false);
            
            // Slot background
            let color = if i == selected_index {
                UIColor::new(1.0, 1.0, 1.0, 0.8) // Highlighted
            } else {
                UIColor::new(0.2, 0.2, 0.2, 0.8) // Normal
            };
            
            ui_renderer.draw_rect(UIRect {
                x,
                y,
                width: SLOT_SIZE,
                height: SLOT_SIZE,
            }, color);
            
            // Slot border
            ui_renderer.draw_rect_outline(UIRect {
                x,
                y,
                width: SLOT_SIZE,
                height: SLOT_SIZE,
            }, UIColor::new(0.5, 0.5, 0.5, 1.0), 2.0);
            
            // Item in slot
            if let Some(slot) = inventory.get_slot(i) {
                if let Some(item) = slot.get_item() {
                    self.render_item(ui_renderer, x, y, item);
                }
            }
            
            // Slot number
            let number = ((i + 1) % 10).to_string();
            ui_renderer.draw_text(&number, x + 4.0, y + 4.0, 12.0, UIColor::WHITE);
        }
    }
    
    /// Render the full inventory
    fn render_full_inventory(&self, ui_renderer: &mut UIRenderer, inventory: &PlayerInventory) {
        // Semi-transparent background
        ui_renderer.draw_rect(UIRect {
            x: self.inventory_position.x - INVENTORY_PADDING,
            y: self.inventory_position.y - INVENTORY_PADDING,
            width: 9.0 * (SLOT_SIZE + SLOT_PADDING) + INVENTORY_PADDING * 2.0,
            height: 4.0 * (SLOT_SIZE + SLOT_PADDING) + INVENTORY_PADDING * 2.0,
        }, UIColor::new(0.0, 0.0, 0.0, 0.8));
        
        // Render all slots
        for i in 0..INVENTORY_SIZE {
            let (x, y) = self.get_slot_position(i, true);
            
            // Slot background
            let color = if Some(i) == self.selected_slot {
                UIColor::new(0.8, 0.8, 0.8, 0.8) // Selected
            } else if i == inventory.selected_hotbar_index() && i < HOTBAR_SIZE {
                UIColor::new(1.0, 1.0, 1.0, 0.8) // Highlighted hotbar
            } else {
                UIColor::new(0.2, 0.2, 0.2, 0.8) // Normal
            };
            
            ui_renderer.draw_rect(UIRect {
                x,
                y,
                width: SLOT_SIZE,
                height: SLOT_SIZE,
            }, color);
            
            // Slot border
            ui_renderer.draw_rect_outline(UIRect {
                x,
                y,
                width: SLOT_SIZE,
                height: SLOT_SIZE,
            }, UIColor::new(0.5, 0.5, 0.5, 1.0), 2.0);
            
            // Item in slot
            if let Some(slot) = inventory.get_slot(i) {
                if let Some(item) = slot.get_item() {
                    self.render_item(ui_renderer, x, y, item);
                }
            }
        }
        
        // Draw separation line between main inventory and hotbar
        let sep_y = self.inventory_position.y + 2.75 * (SLOT_SIZE + SLOT_PADDING);
        ui_renderer.draw_rect(UIRect {
            x: self.inventory_position.x,
            y: sep_y,
            width: 9.0 * (SLOT_SIZE + SLOT_PADDING) - SLOT_PADDING,
            height: 2.0,
        }, UIColor::new(0.5, 0.5, 0.5, 0.8));
    }
    
    /// Render an item in a slot
    fn render_item(&self, ui_renderer: &mut UIRenderer, x: f32, y: f32, item: &crate::inventory::ItemStack) {
        // For now, just render the block ID as text
        // In the future, this would render the actual block texture
        let text = format!("{:?}", item.item_id);
        ui_renderer.draw_text(&text, x + 8.0, y + SLOT_SIZE / 2.0 - 6.0, 14.0, UIColor::WHITE);
        
        // Render item count if > 1
        if item.count > 1 {
            let count_text = item.count.to_string();
            ui_renderer.draw_text(
                &count_text,
                x + SLOT_SIZE - 16.0,
                y + SLOT_SIZE - 16.0,
                16.0,
                UIColor::WHITE
            );
        }
    }
}