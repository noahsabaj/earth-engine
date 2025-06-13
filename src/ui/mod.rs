pub mod data_inventory_ui;
pub mod inventory_input;

// Re-export data-oriented UI system
pub use data_inventory_ui::{
    // Data structures
    InventoryUIData,
    InventoryUIState,
    InventoryInputState,
    InventoryAction,
    
    // Functions
    init_inventory_ui,
    resize_inventory_ui,
    open_inventory_ui,
    close_inventory_ui,
    toggle_inventory_ui,
    is_inventory_open,
    get_slot_position,
    handle_inventory_click,
    render_inventory_ui,
    process_inventory_input,
};

pub use inventory_input::{InventoryInputHandler, MouseButton};

// Legacy OOP-style UI (to be removed)
#[deprecated(note = "Use data_inventory_ui module instead")]
pub mod inventory_ui;
#[allow(deprecated)]
#[deprecated(note = "Use InventoryUIData instead")]
pub use inventory_ui::InventoryUI;