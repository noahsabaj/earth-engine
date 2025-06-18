pub mod data_inventory;
// pub mod drop_handler; // Removed - was using deprecated inventory system

// Re-export data-oriented inventory system
pub use data_inventory::{
    // Data structures
    PlayerInventoryData,
    InventorySlotData,
    ItemStackData,
    SlotType,
    
    // Constants
    HOTBAR_SIZE,
    INVENTORY_SIZE,
    MAX_STACK_SIZE,
    
    // Functions
    init_inventory,
    create_item_stack,
    create_single_item,
    add_item_to_inventory,
    remove_selected_items,
    get_selected_item,
    get_selected_hotbar_index,
    set_selected_hotbar_slot,
    find_item_in_inventory,
    count_items_in_inventory,
    clear_inventory,
    swap_slots,
    set_slot_contents,
    has_empty_slot,
    first_empty_slot,
    get_slot,
    get_slot_mut,
    get_slot_item,
    get_hotbar_slots,
    get_main_slots,
    create_empty_slot,
    create_slot_with_item,
    
    // Batch operations
    InventoryOperationBatch,
    apply_operation_batch,
};

// pub use drop_handler::ItemDropHandler; // Removed with deprecated inventory system

