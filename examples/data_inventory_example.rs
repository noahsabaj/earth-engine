/// Example demonstrating the data-oriented inventory system
/// 
/// This shows how to use the pure data structures and free functions
/// instead of OOP-style methods.

use earth_engine::inventory::*;
use earth_engine::item::ItemId;

fn inventory_example() {
    println!("=== Data-Oriented Inventory System Example ===\n");
    
    // Initialize empty inventory
    let mut inventory = init_inventory();
    println!("Created empty inventory with {} slots", INVENTORY_SIZE);
    println!("Hotbar size: {}", HOTBAR_SIZE);
    
    // Add some items
    println!("\n--- Adding items ---");
    
    // Add 32 dirt blocks
    let dirt_stack = create_item_stack(ItemId::DIRT_BLOCK, 32);
    let remaining = add_item_to_inventory(&mut inventory, dirt_stack);
    match remaining {
        None => println!("Added 32 dirt blocks to inventory"),
        Some(r) => println!("Could only add {} dirt blocks, {} remain", 32 - r.count, r.count),
    }
    
    // Add 64 stone blocks
    let stone_stack = create_item_stack(ItemId::STONE_BLOCK, 64);
    let remaining = add_item_to_inventory(&mut inventory, stone_stack);
    match remaining {
        None => println!("Added 64 stone blocks to inventory"),
        Some(r) => println!("Could only add {} stone blocks, {} remain", 64 - r.count, r.count),
    }
    
    // Add a wooden pickaxe
    let pickaxe = create_single_item(ItemId::WOODEN_PICKAXE);
    add_item_to_inventory(&mut inventory, pickaxe);
    println!("Added wooden pickaxe to inventory");
    
    // Check inventory status
    println!("\n--- Inventory Status ---");
    println!("Selected hotbar slot: {}", get_selected_hotbar_index(&inventory));
    
    if let Some(selected_item) = get_selected_item(&inventory) {
        println!("Selected item: ID={}, Count={}", selected_item.item_id, selected_item.count);
    } else {
        println!("No item selected");
    }
    
    // Count specific items
    let dirt_count = count_items_in_inventory(&inventory, ItemId::DIRT_BLOCK);
    let stone_count = count_items_in_inventory(&inventory, ItemId::STONE_BLOCK);
    println!("Total dirt blocks: {}", dirt_count);
    println!("Total stone blocks: {}", stone_count);
    
    // Find items
    if let Some(index) = find_item_in_inventory(&inventory, ItemId::WOODEN_PICKAXE) {
        println!("Found pickaxe at slot {}", index);
    }
    
    // Change selected slot
    println!("\n--- Changing selected slot ---");
    set_selected_hotbar_slot(&mut inventory, 2);
    println!("Selected hotbar slot changed to: {}", get_selected_hotbar_index(&inventory));
    
    // Remove some items from selected slot
    if let Some(removed) = remove_selected_items(&mut inventory, 10) {
        println!("Removed {} items of type {}", removed.count, removed.item_id);
    }
    
    // Swap slots
    println!("\n--- Swapping slots ---");
    swap_slots(&mut inventory, 0, 1);
    println!("Swapped slots 0 and 1");
    
    // Batch operations
    println!("\n--- Batch operations ---");
    let mut batch = InventoryOperationBatch::default();
    
    // Add some operations to the batch
    batch.adds.push((5, create_item_stack(ItemId::COAL, 16)));
    batch.swaps.push((2, 3));
    batch.removes.push((0, 5));
    
    // Apply the batch
    apply_operation_batch(&mut inventory, &batch);
    println!("Applied batch operations: {} adds, {} swaps, {} removes", 
        batch.adds.len(), batch.swaps.len(), batch.removes.len());
    
    // Check if there are empty slots
    if has_empty_slot(&inventory) {
        if let Some(empty_index) = first_empty_slot(&inventory) {
            println!("\nFirst empty slot is at index {}", empty_index);
        }
    } else {
        println!("\nNo empty slots available");
    }
    
    // Clear inventory
    println!("\n--- Clearing inventory ---");
    clear_inventory(&mut inventory);
    println!("Inventory cleared");
    
    // Verify it's empty
    let total_items: u32 = (0..INVENTORY_SIZE)
        .filter_map(|i| get_slot(&inventory, i))
        .filter_map(|slot| get_slot_item(slot))
        .map(|item| item.count)
        .sum();
    println!("Total items after clearing: {}", total_items);
}

/// Example of how to use the inventory UI
fn ui_example() {
    use earth_engine::ui::*;
    
    println!("\n\n=== Inventory UI Example ===\n");
    
    // Initialize UI
    let mut ui = init_inventory_ui(1920.0, 1080.0);
    let mut inventory = init_inventory();
    
    // Add some items for display
    add_item_to_inventory(&mut inventory, create_item_stack(ItemId::GRASS_BLOCK, 64));
    add_item_to_inventory(&mut inventory, create_item_stack(ItemId::WOOD_BLOCK, 32));
    
    println!("UI State: {:?}", if is_inventory_open(&ui) { "Open" } else { "Closed" });
    
    // Toggle inventory
    ui = toggle_inventory_ui(&ui);
    println!("After toggle: {:?}", if is_inventory_open(&ui) { "Open" } else { "Closed" });
    
    // Simulate mouse click
    let (updated_ui, clicked_slot) = handle_inventory_click(&ui, 960.0, 540.0);
    if let Some(slot) = clicked_slot {
        println!("Clicked on slot {}", slot);
    }
    
    // Process input
    let input = InventoryInputState {
        mouse_x: 960.0,
        mouse_y: 540.0,
        left_click: 1,
        right_click: 0,
        shift_held: 0,
        _padding: 0,
    };
    
    let (updated_ui, action) = process_inventory_input(&ui, &mut inventory, &input);
    if let Some(action) = action {
        match action {
            InventoryAction::SwapSlots(a, b) => println!("Action: Swap slots {} and {}", a, b),
            InventoryAction::QuickMove(slot) => println!("Action: Quick move slot {}", slot),
            InventoryAction::SplitStack(slot) => println!("Action: Split stack at slot {}", slot),
            InventoryAction::DropItem(slot) => println!("Action: Drop item from slot {}", slot),
        }
    }
    
    // Resize UI
    let resized_ui = resize_inventory_ui(&updated_ui, 2560.0, 1440.0);
    println!("UI resized to {}x{}", resized_ui.screen_width, resized_ui.screen_height);
}

fn main() {
    inventory_example();
    ui_example();
}

