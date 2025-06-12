# Data-Oriented Inventory System

This document describes the new data-oriented inventory system that replaces the old OOP-style implementation.

## Overview

The inventory system has been converted to follow strict data-oriented programming principles:
- No methods on structs
- Pure data structures (POD - Plain Old Data)
- Free functions for all operations
- Pre-allocated fixed-size arrays
- GPU-friendly memory layout

## Core Data Structures

### ItemStackData
```rust
#[repr(C)]
pub struct ItemStackData {
    pub item_id: u32,
    pub count: u32,
}
```

### InventorySlotData
```rust
#[repr(C)]
pub struct InventorySlotData {
    pub slot_type: u8,    // SlotType as u8
    pub has_item: u8,     // 0 = empty, 1 = has item
    pub _padding: [u8; 2],
    pub item: ItemStackData,
}
```

### PlayerInventoryData
```rust
#[repr(C)]
pub struct PlayerInventoryData {
    pub slots: [InventorySlotData; INVENTORY_SIZE],
    pub selected_hotbar_slot: u32,
    pub _padding: u32,
}
```

## Key Functions

### Initialization
- `init_inventory()` - Create empty inventory with pre-allocated slots

### Item Operations
- `create_item_stack(item_id, count)` - Create new item stack
- `add_item_to_inventory(&mut inventory, item)` - Add items (merges stacks)
- `remove_selected_items(&mut inventory, count)` - Remove from selected slot

### Slot Operations
- `get_slot(&inventory, index)` - Get slot by index
- `swap_slots(&mut inventory, index1, index2)` - Swap two slots
- `set_slot_contents(&mut inventory, index, item)` - Set slot contents

### Query Operations
- `find_item_in_inventory(&inventory, item_id)` - Find first occurrence
- `count_items_in_inventory(&inventory, item_id)` - Count total items
- `has_empty_slot(&inventory)` - Check for empty slots

### Batch Operations
- `apply_operation_batch(&mut inventory, &batch)` - Apply multiple operations efficiently

## Migration Guide

### Old (OOP) Style:
```rust
let mut inventory = PlayerInventory::new();
inventory.add_item(ItemStack::new(item_id, 32));
let selected = inventory.get_selected_item();
inventory.select_hotbar_slot(2);
```

### New (Data-Oriented) Style:
```rust
let mut inventory = init_inventory();
add_item_to_inventory(&mut inventory, create_item_stack(item_id, 32));
let selected = get_selected_item(&inventory);
set_selected_hotbar_slot(&mut inventory, 2);
```

## Benefits

1. **Performance**: All data is contiguous in memory, improving cache performance
2. **GPU-Friendly**: `#[repr(C)]` structs can be directly uploaded to GPU buffers
3. **Predictable**: No hidden allocations or method dispatch overhead
4. **Composable**: Functions can be easily combined and batched
5. **Thread-Safe**: Pure functions with explicit data dependencies

## Example Usage

See `examples/data_inventory_example.rs` for a complete example.

```rust
use earth_engine::inventory::*;

// Initialize
let mut inventory = init_inventory();

// Add items
let stack = create_item_stack(ItemId::STONE_BLOCK, 64);
add_item_to_inventory(&mut inventory, stack);

// Query
let count = count_items_in_inventory(&inventory, ItemId::STONE_BLOCK);

// Batch operations
let mut batch = InventoryOperationBatch::default();
batch.swaps.push((0, 1));
batch.adds.push((5, create_item_stack(ItemId::COAL, 16)));
apply_operation_batch(&mut inventory, &batch);
```