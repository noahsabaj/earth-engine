//! Legacy inventory input handling (uses deprecated inventory system)
//! Will be updated when inventory system migration is complete.

#![allow(warnings)]

use crate::input::KeyCode;
use crate::ui::inventory_ui::{InventoryUI, InventoryUIState};
use crate::inventory::{PlayerInventory, ItemDropHandler};
use crate::ecs::{EcsWorldData, EntityId};
use glam::Vec3;
use std::collections::HashSet;

/// Handles input for the inventory system
pub struct InventoryInputHandler {
    pressed_keys: HashSet<KeyCode>,
    shift_pressed: bool,
    ctrl_pressed: bool,
}

impl InventoryInputHandler {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            shift_pressed: false,
            ctrl_pressed: false,
        }
    }
    
    /// Handle key press event
    pub fn handle_key_press(&mut self, key: KeyCode) {
        self.pressed_keys.insert(key);
        
        match key {
            KeyCode::ShiftLeft | KeyCode::ShiftRight => self.shift_pressed = true,
            KeyCode::ControlLeft | KeyCode::ControlRight => self.ctrl_pressed = true,
            _ => {}
        }
    }
    
    /// Handle key release event
    pub fn handle_key_release(&mut self, key: KeyCode) {
        self.pressed_keys.remove(&key);
        
        match key {
            KeyCode::ShiftLeft | KeyCode::ShiftRight => self.shift_pressed = false,
            KeyCode::ControlLeft | KeyCode::ControlRight => self.ctrl_pressed = false,
            _ => {}
        }
    }
    
    /// Process inventory-related input
    pub fn update(
        &mut self,
        inventory_ui: &mut InventoryUI,
        inventory: &mut PlayerInventory,
        world: &mut EcsWorldData,
        player_position: Vec3,
        player_forward: Vec3,
    ) {
        // Toggle inventory with E
        if self.pressed_keys.remove(&KeyCode::KeyE) {
            inventory_ui.toggle();
        }
        
        // Close inventory with Escape
        if self.pressed_keys.remove(&KeyCode::Escape) && inventory_ui.is_open() {
            inventory_ui.close();
        }
        
        // Drop items with Q
        if self.pressed_keys.remove(&KeyCode::KeyQ) && !inventory_ui.is_open() {
            if self.ctrl_pressed {
                // Drop entire stack
                ItemDropHandler::drop_selected_stack(
                    inventory,
                    world,
                    player_position,
                    player_forward,
                );
            } else {
                // Drop single item
                ItemDropHandler::drop_selected_item(
                    inventory,
                    world,
                    player_position,
                    player_forward,
                );
            }
        }
        
        // Hotbar selection with number keys
        for i in 1..=9 {
            let key = match i {
                1 => KeyCode::Digit1,
                2 => KeyCode::Digit2,
                3 => KeyCode::Digit3,
                4 => KeyCode::Digit4,
                5 => KeyCode::Digit5,
                6 => KeyCode::Digit6,
                7 => KeyCode::Digit7,
                8 => KeyCode::Digit8,
                9 => KeyCode::Digit9,
                _ => continue,
            };
            
            if self.pressed_keys.remove(&key) && !inventory_ui.is_open() {
                inventory.select_hotbar_slot(i - 1);
            }
        }
    }
    
    /// Handle mouse wheel for hotbar selection
    pub fn handle_scroll(&mut self, delta: f32, inventory: &mut PlayerInventory) {
        let current = inventory.selected_hotbar_index();
        let hotbar_size = crate::inventory::HOTBAR_SIZE;
        
        if delta > 0.0 {
            // Scroll up - previous slot
            let new_index = if current == 0 {
                hotbar_size - 1
            } else {
                current - 1
            };
            inventory.select_hotbar_slot(new_index);
        } else if delta < 0.0 {
            // Scroll down - next slot
            let new_index = (current + 1) % hotbar_size;
            inventory.select_hotbar_slot(new_index);
        }
    }
    
    /// Handle mouse click in inventory
    pub fn handle_inventory_click(
        &mut self,
        inventory_ui: &mut InventoryUI,
        inventory: &mut PlayerInventory,
        mouse_x: f32,
        mouse_y: f32,
        button: MouseButton,
    ) {
        if !inventory_ui.is_open() {
            return;
        }
        
        if let Some(slot_index) = inventory_ui.handle_click(mouse_x, mouse_y) {
            match button {
                MouseButton::Left => {
                    if self.shift_pressed {
                        // Shift-click: quick transfer between inventory sections
                        self.quick_transfer_slot(inventory, slot_index);
                    } else {
                        // Normal click: select/swap slots
                        // This would be implemented with drag-and-drop in a full system
                    }
                }
                MouseButton::Right => {
                    // Right-click: split stack
                    self.split_stack(inventory, slot_index);
                }
                _ => {}
            }
        }
    }
    
    /// Quick transfer items between hotbar and main inventory
    fn quick_transfer_slot(&mut self, inventory: &mut PlayerInventory, slot_index: usize) {
        if slot_index < crate::inventory::HOTBAR_SIZE {
            // Transfer from hotbar to main inventory
            if let Some(slot) = inventory.get_slot_mut(slot_index) {
                if let Some(item) = slot.take_item() {
                    // Try to add to main inventory (skip hotbar)
                    let mut remaining = item;
                    for i in crate::inventory::HOTBAR_SIZE..crate::inventory::INVENTORY_SIZE {
                        if let Some(target_slot) = inventory.get_slot_mut(i) {
                            if let Some(r) = target_slot.try_add_items(remaining) {
                                remaining = r;
                            } else {
                                return; // All items transferred
                            }
                        }
                    }
                    // Put back any remaining items
                    if !remaining.is_empty() {
                        if let Some(slot) = inventory.get_slot_mut(slot_index) {
                            slot.put_item(remaining);
                        }
                    }
                }
            }
        } else {
            // Transfer from main inventory to hotbar
            if let Some(slot) = inventory.get_slot_mut(slot_index) {
                if let Some(item) = slot.take_item() {
                    // Try to add to hotbar
                    let mut remaining = item;
                    for i in 0..crate::inventory::HOTBAR_SIZE {
                        if let Some(target_slot) = inventory.get_slot_mut(i) {
                            if let Some(r) = target_slot.try_add_items(remaining) {
                                remaining = r;
                            } else {
                                return; // All items transferred
                            }
                        }
                    }
                    // Put back any remaining items
                    if !remaining.is_empty() {
                        if let Some(slot) = inventory.get_slot_mut(slot_index) {
                            slot.put_item(remaining);
                        }
                    }
                }
            }
        }
    }
    
    /// Split a stack in half
    fn split_stack(&mut self, inventory: &mut PlayerInventory, slot_index: usize) {
        if let Some(slot) = inventory.get_slot_mut(slot_index) {
            if let Some(item) = slot.get_item_mut() {
                let split_count = item.count / 2;
                if split_count > 0 {
                    if let Some(split_stack) = item.split(split_count) {
                        // Find an empty slot for the split stack
                        if let Some(empty_index) = inventory.first_empty_slot() {
                            if let Some(empty_slot) = inventory.get_slot_mut(empty_index) {
                                empty_slot.put_item(split_stack);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Mouse button enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}