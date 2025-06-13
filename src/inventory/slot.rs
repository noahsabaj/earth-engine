//! Legacy OOP-style inventory slot (deprecated)
//! Use the data_inventory module instead for new code.

#![allow(warnings)]

use super::ItemStack;

/// Type of inventory slot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotType {
    Normal,
    Hotbar,
    // Future slot types: Armor, Tool, etc.
}

/// A single slot in the inventory
#[derive(Debug, Clone)]
pub struct InventorySlot {
    pub slot_type: SlotType,
    pub item: Option<ItemStack>,
}

impl InventorySlot {
    /// Create an empty slot
    pub fn empty(slot_type: SlotType) -> Self {
        Self {
            slot_type,
            item: None,
        }
    }
    
    /// Create a slot with an item
    pub fn with_item(slot_type: SlotType, item: ItemStack) -> Self {
        Self {
            slot_type,
            item: Some(item),
        }
    }
    
    /// Check if slot is empty
    pub fn is_empty(&self) -> bool {
        self.item.is_none()
    }
    
    /// Get the item stack in this slot
    pub fn get_item(&self) -> Option<&ItemStack> {
        self.item.as_ref()
    }
    
    /// Get mutable item stack in this slot
    pub fn get_item_mut(&mut self) -> Option<&mut ItemStack> {
        self.item.as_mut()
    }
    
    /// Take the item from this slot
    pub fn take_item(&mut self) -> Option<ItemStack> {
        self.item.take()
    }
    
    /// Put an item in this slot, returns previous item if any
    pub fn put_item(&mut self, item: ItemStack) -> Option<ItemStack> {
        self.item.replace(item)
    }
    
    /// Try to add items to this slot
    pub fn try_add_items(&mut self, item: ItemStack) -> Option<ItemStack> {
        if let Some(existing) = &mut self.item {
            if existing.item_id == item.item_id {
                let remaining = existing.try_add(item.count);
                if remaining > 0 {
                    Some(ItemStack::new(item.item_id, remaining))
                } else {
                    None
                }
            } else {
                // Different item type, can't add
                Some(item)
            }
        } else {
            // Empty slot, just put the item
            self.item = Some(item);
            None
        }
    }
    
    /// Clean up empty stacks
    pub fn cleanup(&mut self) {
        if let Some(item) = &self.item {
            if item.is_empty() {
                self.item = None;
            }
        }
    }
}