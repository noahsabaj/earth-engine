/// Data-Oriented Inventory UI System
/// 
/// Pure data structures with free functions.
/// No methods, no mutations through self, just data and transformations.

use crate::inventory::{
    PlayerInventoryData, ItemStackData, InventorySlotData,
    get_selected_hotbar_index, get_slot, get_slot_item,
    HOTBAR_SIZE, INVENTORY_SIZE
};
use crate::renderer::ui::{UIRenderer, UIElement, UIRect, UIColor};
use crate::item::ItemId;
use glam::Vec2;
use bytemuck::{Pod, Zeroable};

/// Size of inventory slots in pixels
const SLOT_SIZE: f32 = 48.0;
const SLOT_PADDING: f32 = 4.0;
const INVENTORY_PADDING: f32 = 16.0;

/// State of the inventory UI
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryUIState {
    Closed = 0,
    Open = 1,
}

/// Inventory UI data - plain old data
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct InventoryUIData {
    pub state: u8, // InventoryUIState as u8
    pub selected_slot: u32, // u32::MAX means no selection
    pub hotbar_position_x: f32,
    pub hotbar_position_y: f32,
    pub inventory_position_x: f32,
    pub inventory_position_y: f32,
    pub screen_width: f32,
    pub screen_height: f32,
}

// Pure functions for inventory UI operations

/// Initialize inventory UI with default values
pub fn init_inventory_ui(screen_width: f32, screen_height: f32) -> InventoryUIData {
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
    
    InventoryUIData {
        state: InventoryUIState::Closed as u8,
        selected_slot: u32::MAX,
        hotbar_position_x: hotbar_x,
        hotbar_position_y: hotbar_y,
        inventory_position_x: inventory_x,
        inventory_position_y: inventory_y,
        screen_width,
        screen_height,
    }
}

/// Update screen dimensions
pub fn resize_inventory_ui(ui: &InventoryUIData, screen_width: f32, screen_height: f32) -> InventoryUIData {
    let mut updated = *ui;
    updated.screen_width = screen_width;
    updated.screen_height = screen_height;
    
    // Recalculate positions
    let hotbar_width = HOTBAR_SIZE as f32 * (SLOT_SIZE + SLOT_PADDING);
    updated.hotbar_position_x = (screen_width - hotbar_width) / 2.0;
    updated.hotbar_position_y = screen_height - SLOT_SIZE - INVENTORY_PADDING * 2.0;
    
    let inventory_cols = 9;
    let inventory_rows = 4;
    let inventory_width = inventory_cols as f32 * (SLOT_SIZE + SLOT_PADDING);
    let inventory_height = inventory_rows as f32 * (SLOT_SIZE + SLOT_PADDING);
    updated.inventory_position_x = (screen_width - inventory_width) / 2.0;
    updated.inventory_position_y = (screen_height - inventory_height) / 2.0;
    
    updated
}

/// Open the inventory
pub fn open_inventory_ui(ui: &InventoryUIData) -> InventoryUIData {
    let mut updated = *ui;
    updated.state = InventoryUIState::Open as u8;
    updated
}

/// Close the inventory
pub fn close_inventory_ui(ui: &InventoryUIData) -> InventoryUIData {
    let mut updated = *ui;
    updated.state = InventoryUIState::Closed as u8;
    updated.selected_slot = u32::MAX;
    updated
}

/// Toggle inventory open/closed
pub fn toggle_inventory_ui(ui: &InventoryUIData) -> InventoryUIData {
    if ui.state == InventoryUIState::Open as u8 {
        close_inventory_ui(ui)
    } else {
        open_inventory_ui(ui)
    }
}

/// Check if inventory is open
pub fn is_inventory_open(ui: &InventoryUIData) -> bool {
    ui.state == InventoryUIState::Open as u8
}

/// Get the position of a slot
pub fn get_slot_position(ui: &InventoryUIData, index: usize, full_inventory: bool) -> (f32, f32) {
    if index < HOTBAR_SIZE {
        // Hotbar slot
        let x = ui.hotbar_position_x + index as f32 * (SLOT_SIZE + SLOT_PADDING);
        let y = if full_inventory {
            // Show in full inventory view
            ui.inventory_position_y + 3.0 * (SLOT_SIZE + SLOT_PADDING)
        } else {
            // Show at bottom of screen
            ui.hotbar_position_y
        };
        (x, y)
    } else {
        // Main inventory slot
        let adjusted_index = index - HOTBAR_SIZE;
        let col = adjusted_index % 9;
        let row = adjusted_index / 9;
        let x = ui.inventory_position_x + col as f32 * (SLOT_SIZE + SLOT_PADDING);
        let y = ui.inventory_position_y + row as f32 * (SLOT_SIZE + SLOT_PADDING);
        (x, y)
    }
}

/// Handle mouse click
pub fn handle_inventory_click(ui: &InventoryUIData, mouse_x: f32, mouse_y: f32) -> (InventoryUIData, Option<usize>) {
    if ui.state != InventoryUIState::Open as u8 {
        return (*ui, None);
    }
    
    // Check inventory slots
    for i in 0..INVENTORY_SIZE {
        let (x, y) = get_slot_position(ui, i, true);
        let rect = UIRect {
            x,
            y,
            width: SLOT_SIZE,
            height: SLOT_SIZE,
        };
        
        if rect.contains(mouse_x, mouse_y) {
            let mut updated = *ui;
            updated.selected_slot = i as u32;
            return (updated, Some(i));
        }
    }
    
    (*ui, None)
}

/// Render the inventory UI
pub fn render_inventory_ui(ui: &InventoryUIData, ui_renderer: &mut UIRenderer, inventory: &PlayerInventoryData) {
    match ui.state {
        0 => { // Closed
            // Only render hotbar
            render_hotbar(ui, ui_renderer, inventory);
        }
        1 => { // Open
            // Render full inventory
            render_full_inventory(ui, ui_renderer, inventory);
        }
        _ => {} // Invalid state
    }
}

/// Render just the hotbar
fn render_hotbar(ui: &InventoryUIData, ui_renderer: &mut UIRenderer, inventory: &PlayerInventoryData) {
    let selected_index = get_selected_hotbar_index(inventory);
    
    for i in 0..HOTBAR_SIZE {
        let (x, y) = get_slot_position(ui, i, false);
        
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
        if let Some(slot) = get_slot(inventory, i) {
            if let Some(item) = get_slot_item(slot) {
                render_item(ui_renderer, x, y, &item);
            }
        }
        
        // Slot number
        let number = ((i + 1) % 10).to_string();
        ui_renderer.draw_text(&number, x + 4.0, y + 4.0, 12.0, UIColor::WHITE);
    }
}

/// Render the full inventory
fn render_full_inventory(ui: &InventoryUIData, ui_renderer: &mut UIRenderer, inventory: &PlayerInventoryData) {
    // Semi-transparent background
    ui_renderer.draw_rect(UIRect {
        x: ui.inventory_position_x - INVENTORY_PADDING,
        y: ui.inventory_position_y - INVENTORY_PADDING,
        width: 9.0 * (SLOT_SIZE + SLOT_PADDING) + INVENTORY_PADDING * 2.0,
        height: 4.0 * (SLOT_SIZE + SLOT_PADDING) + INVENTORY_PADDING * 2.0,
    }, UIColor::new(0.0, 0.0, 0.0, 0.8));
    
    // Render all slots
    for i in 0..INVENTORY_SIZE {
        let (x, y) = get_slot_position(ui, i, true);
        
        // Slot background
        let color = if ui.selected_slot != u32::MAX && i == ui.selected_slot as usize {
            UIColor::new(0.8, 0.8, 0.8, 0.8) // Selected
        } else if i == get_selected_hotbar_index(inventory) && i < HOTBAR_SIZE {
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
        if let Some(slot) = get_slot(inventory, i) {
            if let Some(item) = get_slot_item(slot) {
                render_item(ui_renderer, x, y, &item);
            }
        }
    }
    
    // Draw separation line between main inventory and hotbar
    let sep_y = ui.inventory_position_y + 2.75 * (SLOT_SIZE + SLOT_PADDING);
    ui_renderer.draw_rect(UIRect {
        x: ui.inventory_position_x,
        y: sep_y,
        width: 9.0 * (SLOT_SIZE + SLOT_PADDING) - SLOT_PADDING,
        height: 2.0,
    }, UIColor::new(0.5, 0.5, 0.5, 0.8));
}

/// Render an item in a slot
fn render_item(ui_renderer: &mut UIRenderer, x: f32, y: f32, item: &ItemStackData) {
    // For now, just render the item ID as text
    // In the future, this would render the actual item texture
    let text = format!("{}", item.item_id);
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

/// UI input state for inventory
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct InventoryInputState {
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub left_click: u8,
    pub right_click: u8,
    pub shift_held: u8,
    pub _padding: u8,
}

/// Process inventory input batch
pub fn process_inventory_input(
    ui: &InventoryUIData, 
    inventory: &mut PlayerInventoryData,
    input: &InventoryInputState
) -> (InventoryUIData, Option<InventoryAction>) {
    if !is_inventory_open(ui) {
        return (*ui, None);
    }
    
    if input.left_click != 0 {
        let (updated_ui, clicked_slot) = handle_inventory_click(ui, input.mouse_x, input.mouse_y);
        
        if let Some(slot_index) = clicked_slot {
            // Handle slot interaction
            if input.shift_held != 0 {
                // Shift-click: quick move
                return (updated_ui, Some(InventoryAction::QuickMove(slot_index)));
            } else if ui.selected_slot != u32::MAX {
                // Swap with selected slot
                let selected = ui.selected_slot as usize;
                return (updated_ui, Some(InventoryAction::SwapSlots(selected, slot_index)));
            } else {
                // Just select the slot
                return (updated_ui, None);
            }
        }
        
        return (updated_ui, None);
    }
    
    (*ui, None)
}

/// Inventory actions that can be performed
#[derive(Debug, Clone, Copy)]
pub enum InventoryAction {
    SwapSlots(usize, usize),
    QuickMove(usize),
    SplitStack(usize),
    DropItem(usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ui_initialization() {
        let ui = init_inventory_ui(1920.0, 1080.0);
        assert_eq!(ui.state, InventoryUIState::Closed as u8);
        assert_eq!(ui.selected_slot, u32::MAX);
        assert_eq!(ui.screen_width, 1920.0);
        assert_eq!(ui.screen_height, 1080.0);
    }
    
    #[test]
    fn test_ui_toggle() {
        let ui = init_inventory_ui(1920.0, 1080.0);
        let opened = toggle_inventory_ui(&ui);
        assert_eq!(opened.state, InventoryUIState::Open as u8);
        
        let closed = toggle_inventory_ui(&opened);
        assert_eq!(closed.state, InventoryUIState::Closed as u8);
    }
    
    #[test]
    fn test_slot_positions() {
        let ui = init_inventory_ui(1920.0, 1080.0);
        
        // Test hotbar positions
        let (x0, y0) = get_slot_position(&ui, 0, false);
        let (x1, y1) = get_slot_position(&ui, 1, false);
        assert_eq!(y0, y1); // Same row
        assert!(x1 > x0); // Next column
        
        // Test main inventory positions
        let (mx0, my0) = get_slot_position(&ui, HOTBAR_SIZE, true);
        let (mx1, my1) = get_slot_position(&ui, HOTBAR_SIZE + 9, true);
        assert!(my1 > my0); // Next row
        assert_eq!(mx0, mx1); // Same column
    }
}