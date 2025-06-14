/// Data-Oriented Inventory System
/// 
/// Pure data structures with free functions.
/// No methods, no mutations through self, just data and transformations.

use crate::item::ItemId;
use bytemuck::{Pod, Zeroable};
use serde::{Serialize, Deserialize};

/// Size of the hotbar
pub const HOTBAR_SIZE: usize = 9;
/// Total inventory size (including hotbar)
pub const INVENTORY_SIZE: usize = 36; // 9 hotbar + 27 main inventory

/// Maximum items in a single stack
pub const MAX_STACK_SIZE: u32 = 64;

/// Type of inventory slot
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotType {
    Normal = 0,
    Hotbar = 1,
    // Future slot types: Armor, Tool, etc.
}

/// Item stack data - plain old data
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct ItemStackData {
    pub item_id: u32, // Assuming ItemId can be represented as u32
    pub count: u32,
}

/// Inventory slot data - plain old data
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct InventorySlotData {
    pub slot_type: u8, // SlotType as u8
    pub has_item: u8,  // 0 = empty, 1 = has item
    pub _padding: [u8; 2],
    pub item: ItemStackData,
}

/// Player inventory data - fixed size arrays, no dynamic allocation
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PlayerInventoryData {
    pub slots: [InventorySlotData; INVENTORY_SIZE],
    pub selected_hotbar_slot: u32,
    pub _padding: u32,
}

impl Default for PlayerInventoryData {
    fn default() -> Self {
        init_inventory()
    }
}

// Pure functions for item stack operations

/// Create a new item stack
pub fn create_item_stack(item_id: ItemId, count: u32) -> ItemStackData {
    ItemStackData {
        item_id: item_id.0, // Assuming ItemId(u32)
        count: count.min(MAX_STACK_SIZE),
    }
}

/// Create a single item
pub fn create_single_item(item_id: ItemId) -> ItemStackData {
    create_item_stack(item_id, 1)
}

/// Check if stack is empty
pub fn is_stack_empty(stack: &ItemStackData) -> bool {
    stack.count == 0
}

/// Check if stack is full
pub fn is_stack_full(stack: &ItemStackData) -> bool {
    stack.count >= MAX_STACK_SIZE
}

/// Check if two stacks can merge
pub fn can_merge_stacks(stack1: &ItemStackData, stack2: &ItemStackData) -> bool {
    stack1.item_id == stack2.item_id && stack1.count < MAX_STACK_SIZE
}

/// Try to add items to a stack, returns remaining items
pub fn try_add_to_stack(stack: &mut ItemStackData, count: u32) -> u32 {
    let space = MAX_STACK_SIZE - stack.count;
    let to_add = count.min(space);
    stack.count += to_add;
    count - to_add
}

/// Split a stack, taking up to the specified count
pub fn split_stack(stack: &mut ItemStackData, count: u32) -> Option<ItemStackData> {
    if count >= stack.count {
        // Take the whole stack
        let result = *stack;
        stack.count = 0;
        Some(result)
    } else if count > 0 {
        // Split the stack
        stack.count -= count;
        Some(create_item_stack(ItemId(stack.item_id), count))
    } else {
        None
    }
}

// Pure functions for slot operations

/// Create an empty slot
pub fn create_empty_slot(slot_type: SlotType) -> InventorySlotData {
    InventorySlotData {
        slot_type: slot_type as u8,
        has_item: 0,
        _padding: [0; 2],
        item: ItemStackData { item_id: 0, count: 0 },
    }
}

/// Create a slot with an item
pub fn create_slot_with_item(slot_type: SlotType, item: ItemStackData) -> InventorySlotData {
    InventorySlotData {
        slot_type: slot_type as u8,
        has_item: 1,
        _padding: [0; 2],
        item,
    }
}

/// Check if slot is empty
pub fn is_slot_empty(slot: &InventorySlotData) -> bool {
    slot.has_item == 0
}

/// Get item from slot
pub fn get_slot_item(slot: &InventorySlotData) -> Option<ItemStackData> {
    if slot.has_item != 0 {
        Some(slot.item)
    } else {
        None
    }
}

/// Take item from slot
pub fn take_from_slot(slot: &mut InventorySlotData) -> Option<ItemStackData> {
    if slot.has_item != 0 {
        let item = slot.item;
        slot.has_item = 0;
        slot.item = ItemStackData { item_id: 0, count: 0 };
        Some(item)
    } else {
        None
    }
}

/// Put item in slot, returns previous item if any
pub fn put_in_slot(slot: &mut InventorySlotData, item: ItemStackData) -> Option<ItemStackData> {
    let previous = take_from_slot(slot);
    slot.has_item = 1;
    slot.item = item;
    previous
}

/// Try to add items to slot
pub fn try_add_to_slot(slot: &mut InventorySlotData, item: ItemStackData) -> Option<ItemStackData> {
    if slot.has_item != 0 {
        if slot.item.item_id == item.item_id {
            let remaining = try_add_to_stack(&mut slot.item, item.count);
            if remaining > 0 {
                Some(create_item_stack(ItemId(item.item_id), remaining))
            } else {
                None
            }
        } else {
            // Different item type, can't add
            Some(item)
        }
    } else {
        // Empty slot, just put the item
        put_in_slot(slot, item);
        None
    }
}

/// Clean up empty stacks in slot
pub fn cleanup_slot(slot: &mut InventorySlotData) {
    if slot.has_item != 0 && is_stack_empty(&slot.item) {
        slot.has_item = 0;
        slot.item = ItemStackData { item_id: 0, count: 0 };
    }
}

// Pure functions for inventory operations

/// Initialize empty inventory
pub fn init_inventory() -> PlayerInventoryData {
    let mut inventory = PlayerInventoryData {
        slots: [create_empty_slot(SlotType::Normal); INVENTORY_SIZE],
        selected_hotbar_slot: 0,
        _padding: 0,
    };
    
    // Set hotbar slots
    for i in 0..HOTBAR_SIZE {
        inventory.slots[i] = create_empty_slot(SlotType::Hotbar);
    }
    
    inventory
}

/// Get selected hotbar slot index
pub fn get_selected_hotbar_index(inventory: &PlayerInventoryData) -> usize {
    inventory.selected_hotbar_slot as usize
}

/// Set selected hotbar slot
pub fn set_selected_hotbar_slot(inventory: &mut PlayerInventoryData, index: usize) {
    if index < HOTBAR_SIZE {
        inventory.selected_hotbar_slot = index as u32;
    }
}

/// Get selected item
pub fn get_selected_item(inventory: &PlayerInventoryData) -> Option<ItemStackData> {
    let index = inventory.selected_hotbar_slot as usize;
    get_slot_item(&inventory.slots[index])
}

/// Get slot by index
pub fn get_slot(inventory: &PlayerInventoryData, index: usize) -> Option<&InventorySlotData> {
    inventory.slots.get(index)
}

/// Get mutable slot by index
pub fn get_slot_mut(inventory: &mut PlayerInventoryData, index: usize) -> Option<&mut InventorySlotData> {
    inventory.slots.get_mut(index)
}

/// Get hotbar slots
pub fn get_hotbar_slots(inventory: &PlayerInventoryData) -> &[InventorySlotData] {
    &inventory.slots[0..HOTBAR_SIZE]
}

/// Get main inventory slots (excluding hotbar)
pub fn get_main_slots(inventory: &PlayerInventoryData) -> &[InventorySlotData] {
    &inventory.slots[HOTBAR_SIZE..]
}

/// Try to add item to inventory
pub fn add_item_to_inventory(inventory: &mut PlayerInventoryData, item: ItemStackData) -> Option<ItemStackData> {
    let mut remaining = item;
    
    // First try to merge with existing stacks in hotbar
    for i in 0..HOTBAR_SIZE {
        if let Some(r) = try_add_to_slot(&mut inventory.slots[i], remaining) {
            remaining = r;
        } else {
            return None; // All items added
        }
    }
    
    // Then try main inventory
    for i in HOTBAR_SIZE..INVENTORY_SIZE {
        if let Some(r) = try_add_to_slot(&mut inventory.slots[i], remaining) {
            remaining = r;
        } else {
            return None; // All items added
        }
    }
    
    // Return any items that couldn't be added
    Some(remaining)
}

/// Remove items from selected hotbar slot
pub fn remove_selected_items(inventory: &mut PlayerInventoryData, count: u32) -> Option<ItemStackData> {
    let index = inventory.selected_hotbar_slot as usize;
    if let Some(slot) = inventory.slots.get_mut(index) {
        if slot.has_item != 0 {
            let removed = split_stack(&mut slot.item, count);
            cleanup_slot(slot);
            removed
        } else {
            None
        }
    } else {
        None
    }
}

/// Find first slot containing a specific item type
pub fn find_item_in_inventory(inventory: &PlayerInventoryData, item_id: ItemId) -> Option<usize> {
    inventory.slots.iter().position(|slot| {
        if let Some(item) = get_slot_item(slot) {
            item.item_id == item_id.0
        } else {
            false
        }
    })
}

/// Count total items of a specific type
pub fn count_items_in_inventory(inventory: &PlayerInventoryData, item_id: ItemId) -> u32 {
    inventory.slots.iter()
        .filter_map(|slot| get_slot_item(slot))
        .filter(|item| item.item_id == item_id.0)
        .map(|item| item.count)
        .sum()
}

/// Clear all items from inventory
pub fn clear_inventory(inventory: &mut PlayerInventoryData) {
    for i in 0..INVENTORY_SIZE {
        take_from_slot(&mut inventory.slots[i]);
    }
}

/// Check if inventory has any empty slots
pub fn has_empty_slot(inventory: &PlayerInventoryData) -> bool {
    inventory.slots.iter().any(|slot| is_slot_empty(slot))
}

/// Get first empty slot index
pub fn first_empty_slot(inventory: &PlayerInventoryData) -> Option<usize> {
    inventory.slots.iter().position(|slot| is_slot_empty(slot))
}

/// Swap items between two slots
pub fn swap_slots(inventory: &mut PlayerInventoryData, index1: usize, index2: usize) {
    if index1 < INVENTORY_SIZE && index2 < INVENTORY_SIZE && index1 != index2 {
        let item1 = take_from_slot(&mut inventory.slots[index1]);
        let item2 = take_from_slot(&mut inventory.slots[index2]);
        
        if let Some(item) = item2 {
            put_in_slot(&mut inventory.slots[index1], item);
        }
        if let Some(item) = item1 {
            put_in_slot(&mut inventory.slots[index2], item);
        }
    }
}

/// Set a specific slot's contents
pub fn set_slot_contents(inventory: &mut PlayerInventoryData, index: usize, item: Option<ItemStackData>) {
    if let Some(slot) = inventory.slots.get_mut(index) {
        if let Some(item_stack) = item {
            put_in_slot(slot, item_stack);
        } else {
            take_from_slot(slot);
        }
    }
}

/// Batch inventory operations for efficiency
pub struct InventoryOperationBatch {
    pub adds: Vec<(usize, ItemStackData)>,
    pub removes: Vec<(usize, u32)>,
    pub swaps: Vec<(usize, usize)>,
}

impl Default for InventoryOperationBatch {
    fn default() -> Self {
        Self {
            adds: Vec::new(),
            removes: Vec::new(),
            swaps: Vec::new(),
        }
    }
}

/// Apply batch of operations to inventory
pub fn apply_operation_batch(inventory: &mut PlayerInventoryData, batch: &InventoryOperationBatch) {
    // Apply removes first
    for &(index, count) in &batch.removes {
        if let Some(slot) = inventory.slots.get_mut(index) {
            if slot.has_item != 0 {
                split_stack(&mut slot.item, count);
                cleanup_slot(slot);
            }
        }
    }
    
    // Apply swaps
    for &(index1, index2) in &batch.swaps {
        swap_slots(inventory, index1, index2);
    }
    
    // Apply adds last
    for &(index, item) in &batch.adds {
        if let Some(slot) = inventory.slots.get_mut(index) {
            try_add_to_slot(slot, item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_inventory_initialization() {
        let inventory = init_inventory();
        assert_eq!(inventory.selected_hotbar_slot, 0);
        
        // Check all slots are empty
        for i in 0..INVENTORY_SIZE {
            assert!(is_slot_empty(&inventory.slots[i]));
        }
        
        // Check hotbar slots have correct type
        for i in 0..HOTBAR_SIZE {
            assert_eq!(inventory.slots[i].slot_type, SlotType::Hotbar as u8);
        }
        
        // Check main slots have correct type
        for i in HOTBAR_SIZE..INVENTORY_SIZE {
            assert_eq!(inventory.slots[i].slot_type, SlotType::Normal as u8);
        }
    }
    
    #[test]
    fn test_add_item() {
        let mut inventory = init_inventory();
        let item = create_item_stack(ItemId(1), 10);
        
        let remaining = add_item_to_inventory(&mut inventory, item);
        assert!(remaining.is_none());
        
        // Check item was added to first slot
        let slot_item = get_slot_item(&inventory.slots[0]).expect("First slot should contain item after adding");
        assert_eq!(slot_item.item_id, 1);
        assert_eq!(slot_item.count, 10);
    }
    
    #[test]
    fn test_stack_merging() {
        let mut inventory = init_inventory();
        
        // Add first stack
        let item1 = create_item_stack(ItemId(1), 32);
        add_item_to_inventory(&mut inventory, item1);
        
        // Add second stack of same item
        let item2 = create_item_stack(ItemId(1), 32);
        let remaining = add_item_to_inventory(&mut inventory, item2);
        
        assert!(remaining.is_none());
        
        // Check first slot is full
        let slot_item = get_slot_item(&inventory.slots[0]).expect("First slot should contain item after stacking");
        assert_eq!(slot_item.count, 64);
    }
}