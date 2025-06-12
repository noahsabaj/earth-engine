pub mod data_inventory;
pub mod drop_handler;

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
    
    // Batch operations
    InventoryOperationBatch,
    apply_operation_batch,
};

pub use drop_handler::ItemDropHandler;

// Legacy OOP-style modules (to be removed)
#[deprecated(note = "Use data_inventory module instead")]
pub mod item;
#[deprecated(note = "Use data_inventory module instead")]
pub mod player_inventory;
#[deprecated(note = "Use data_inventory module instead")]
pub mod slot;

// Legacy exports for compatibility (to be removed)
#[deprecated(note = "Use ItemStackData instead")]
pub use item::ItemStack;
#[deprecated(note = "Use PlayerInventoryData instead")]
pub use player_inventory::PlayerInventory;
#[deprecated(note = "Use InventorySlotData instead")]
pub use slot::InventorySlot;