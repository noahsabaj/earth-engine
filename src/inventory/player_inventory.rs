//! Legacy OOP-style player inventory (deprecated)
//! Use the data_inventory module instead for new code.

#![allow(warnings)]

use super::slot::{InventorySlot, SlotType};
use super::item::ItemStack;
use crate::item::ItemId;

/// Size of the hotbar
pub const HOTBAR_SIZE: usize = 9;
/// Total inventory size (including hotbar)
pub const INVENTORY_SIZE: usize = 36; // 9 hotbar + 27 main inventory

/// Player's inventory
#[derive(Debug, Clone)]
pub struct PlayerInventory {
    slots: Vec<InventorySlot>,
    selected_hotbar_slot: usize,
}

impl PlayerInventory {
    /// Create a new empty inventory
    pub fn new() -> Self {
        let mut slots = Vec::with_capacity(INVENTORY_SIZE);
        
        // Create hotbar slots
        for _ in 0..HOTBAR_SIZE {
            slots.push(InventorySlot::empty(SlotType::Hotbar));
        }
        
        // Create main inventory slots
        for _ in HOTBAR_SIZE..INVENTORY_SIZE {
            slots.push(InventorySlot::empty(SlotType::Normal));
        }
        
        Self {
            slots,
            selected_hotbar_slot: 0,
        }
    }
    
    /// Get the currently selected hotbar slot index
    pub fn selected_hotbar_index(&self) -> usize {
        self.selected_hotbar_slot
    }
    
    /// Set the selected hotbar slot
    pub fn select_hotbar_slot(&mut self, index: usize) {
        if index < HOTBAR_SIZE {
            self.selected_hotbar_slot = index;
        }
    }
    
    /// Get the currently selected item
    pub fn get_selected_item(&self) -> Option<&ItemStack> {
        self.slots[self.selected_hotbar_slot].get_item()
    }
    
    /// Get a slot by index
    pub fn get_slot(&self, index: usize) -> Option<&InventorySlot> {
        self.slots.get(index)
    }
    
    /// Get a mutable slot by index
    pub fn get_slot_mut(&mut self, index: usize) -> Option<&mut InventorySlot> {
        self.slots.get_mut(index)
    }
    
    /// Get hotbar slots
    pub fn hotbar_slots(&self) -> &[InventorySlot] {
        &self.slots[0..HOTBAR_SIZE]
    }
    
    /// Get main inventory slots (excluding hotbar)
    pub fn main_slots(&self) -> &[InventorySlot] {
        &self.slots[HOTBAR_SIZE..]
    }
    
    /// Try to add an item to the inventory
    pub fn add_item(&mut self, item: ItemStack) -> Option<ItemStack> {
        let mut remaining = item;
        
        // First try to merge with existing stacks in hotbar
        for slot in &mut self.slots[0..HOTBAR_SIZE] {
            if let Some(r) = slot.try_add_items(remaining) {
                remaining = r;
            } else {
                return None; // All items added
            }
        }
        
        // Then try main inventory
        for slot in &mut self.slots[HOTBAR_SIZE..] {
            if let Some(r) = slot.try_add_items(remaining) {
                remaining = r;
            } else {
                return None; // All items added
            }
        }
        
        // Return any items that couldn't be added
        Some(remaining)
    }
    
    /// Remove items from the selected hotbar slot
    pub fn remove_selected(&mut self, count: u32) -> Option<ItemStack> {
        if let Some(slot) = self.slots.get_mut(self.selected_hotbar_slot) {
            if let Some(item) = slot.get_item_mut() {
                let removed = item.split(count);
                slot.cleanup();
                removed
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Find the first slot containing a specific item type
    pub fn find_item(&self, item_id: ItemId) -> Option<usize> {
        self.slots.iter().position(|slot| {
            slot.get_item().map_or(false, |item| item.item_id == item_id)
        })
    }
    
    /// Count total items of a specific type
    pub fn count_items(&self, item_id: ItemId) -> u32 {
        self.slots.iter()
            .filter_map(|slot| slot.get_item())
            .filter(|item| item.item_id == item_id)
            .map(|item| item.count)
            .sum()
    }
    
    /// Clear all items from inventory
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            slot.take_item();
        }
    }
    
    /// Check if inventory has any empty slots
    pub fn has_empty_slot(&self) -> bool {
        self.slots.iter().any(|slot| slot.is_empty())
    }
    
    /// Get the first empty slot index
    pub fn first_empty_slot(&self) -> Option<usize> {
        self.slots.iter().position(|slot| slot.is_empty())
    }
    
    /// Swap items between two slots
    pub fn swap_slots(&mut self, index1: usize, index2: usize) {
        if index1 < self.slots.len() && index2 < self.slots.len() && index1 != index2 {
            let item1 = self.slots[index1].take_item();
            let item2 = self.slots[index2].take_item();
            
            if let Some(item) = item2 {
                self.slots[index1].put_item(item);
            }
            if let Some(item) = item1 {
                self.slots[index2].put_item(item);
            }
        }
    }
    
    /// Get all items in the inventory (for serialization)
    pub fn get_all_items(&self) -> Vec<Option<ItemStack>> {
        self.slots.iter()
            .map(|slot| slot.get_item().cloned())
            .collect()
    }
    
    /// Get the hotbar slot indices (for serialization)
    /// Returns an array of 9 indices representing the hotbar slots
    pub fn get_hotbar_indices(&self) -> [usize; 9] {
        // In this implementation, hotbar slots are always indices 0-8
        let mut indices = [0; 9];
        for i in 0..9 {
            indices[i] = i;
        }
        indices
    }
    
    /// Get the currently selected slot index
    pub fn get_selected_slot(&self) -> usize {
        self.selected_hotbar_slot
    }
    
    /// Set a specific slot's contents
    pub fn set_slot(&mut self, index: usize, item: Option<ItemStack>) {
        if let Some(slot) = self.slots.get_mut(index) {
            if let Some(item_stack) = item {
                slot.put_item(item_stack);
            } else {
                slot.take_item();
            }
        }
    }
    
    /// Set the hotbar indices (for deserialization)
    /// Note: In this implementation, hotbar is always slots 0-8, so this is a no-op
    pub fn set_hotbar_indices(&mut self, _indices: [usize; 9]) {
        // Hotbar indices are fixed in this implementation
        // This method exists for compatibility with serialization
    }
    
    /// Set the selected hotbar slot
    pub fn set_selected_slot(&mut self, index: usize) {
        self.select_hotbar_slot(index);
    }
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self::new()
    }
}