use crate::crafting::{RecipeRegistry, CraftingGrid, clear_crafting_grid};
use crate::inventory::ItemStackData;
use crate::item::{ItemRegistry, ItemId};
use crate::renderer::ui::{UIRenderer, UIRect, UIColor};
use glam::Vec2;

/// Result of a crafting operation
#[derive(Debug, Clone)]
pub struct CraftingResult {
    pub output: ItemStackData,
    pub consumes: Vec<(usize, u32)>, // (grid_index, count_to_consume)
}

/// UI for the crafting table
pub struct CraftingTableUI {
    /// The crafting grid (3x3 for crafting table)
    grid: CraftingGrid,
    /// Current crafting result
    current_result: Option<CraftingResult>,
    /// UI position
    position: Vec2,
    /// Whether the UI is open
    is_open: bool,
}

/// Create a new crafting table UI
/// Pure function - returns UI data structure
pub fn create_crafting_table_ui(screen_width: f32, screen_height: f32) -> CraftingTableUI {
    let grid_size = 3;
    let ui_width = (grid_size + 2) as f32 * 52.0 + 100.0; // Grid + result + padding
    let ui_height = grid_size as f32 * 52.0 + 200.0; // Grid + inventory + padding
    
    CraftingTableUI {
        grid: CraftingGrid::new(grid_size, grid_size),
        current_result: None,
        position: Vec2::new(
            (screen_width - ui_width) / 2.0,
            (screen_height - ui_height) / 2.0,
        ),
        is_open: false,
    }
}

/// Open the crafting table UI
/// Function - transforms UI data by setting open state
pub fn open_crafting_table_ui(ui: &mut CraftingTableUI) {
    ui.is_open = true;
}

/// Close the crafting table UI
/// Function - transforms UI data by closing and clearing
pub fn close_crafting_table_ui(ui: &mut CraftingTableUI) {
    ui.is_open = false;
    // Return items to inventory when closing
    // In a real implementation, this would drop items or return to inventory
    clear_crafting_grid(&mut ui.grid);
    ui.current_result = None;
}

/// Check if the UI is open
/// Pure function - reads UI open state
pub fn is_crafting_table_ui_open(ui: &CraftingTableUI) -> bool {
    ui.is_open
}

/// Update the crafting result based on current grid
/// Function - transforms UI data by updating crafting result
pub fn update_crafting_result(ui: &mut CraftingTableUI, recipe_registry: &RecipeRegistry) {
    if let Some(recipe) = recipe_registry.find_recipe(&ui.grid) {
        match &recipe.recipe_type {
            crate::crafting::RecipeType::Shaped(shaped) => {
                ui.current_result = Some(CraftingResult {
                    output: shaped.result.clone(),
                    consumes: calculate_crafting_consumes(ui),
                });
            }
            crate::crafting::RecipeType::Shapeless(shapeless) => {
                ui.current_result = Some(CraftingResult {
                    output: shapeless.result.clone(),
                    consumes: calculate_crafting_consumes(ui),
                });
            }
            _ => {
                ui.current_result = None;
            }
        }
    } else {
        ui.current_result = None;
    }
}

/// Calculate what items need to be consumed for crafting
/// Pure function - calculates consumption list from UI data
fn calculate_crafting_consumes(ui: &CraftingTableUI) -> Vec<(usize, u32)> {
    let mut consumes = Vec::new();
    
    for (index, slot) in ui.grid.slots.iter().enumerate() {
        if slot.is_some() {
            consumes.push((index, 1)); // Consume 1 of each item
        }
    }
    
    consumes
}

/// Place an item in the crafting grid
/// Function - transforms UI grid data by placing item
pub fn place_item_in_crafting_grid(ui: &mut CraftingTableUI, grid_index: usize, item: ItemStackData) -> Option<ItemStackData> {
    if grid_index < ui.grid.slots.len() {
        let old = ui.grid.slots[grid_index].take();
        ui.grid.slots[grid_index] = Some(item);
        old
    } else {
        Some(item) // Invalid index, return the item
    }
}

/// Take an item from the crafting grid
/// Function - transforms UI grid data by removing item
pub fn take_item_from_crafting_grid(ui: &mut CraftingTableUI, grid_index: usize) -> Option<ItemStackData> {
    if grid_index < ui.grid.slots.len() {
        ui.grid.slots[grid_index].take()
    } else {
        None
    }
}

/// Craft the current result
/// Function - transforms UI data by performing crafting operation
pub fn craft_from_table(ui: &mut CraftingTableUI) -> Option<ItemStackData> {
    if let Some(result) = &ui.current_result {
        // Consume items from grid
        for &(index, count) in &result.consumes {
            if let Some(item) = &mut ui.grid.slots[index] {
                if item.count > count {
                    item.count -= count;
                } else {
                    ui.grid.slots[index] = None;
                }
            }
        }
        
        // Clear the result
        let output = result.output.clone();
        ui.current_result = None;
        
        Some(output)
    } else {
        None
    }
}

/// Handle mouse click on the UI
/// Function - processes UI interaction and returns result
pub fn handle_crafting_ui_click(ui: &CraftingTableUI, mouse_x: f32, mouse_y: f32) -> CraftingClickResult {
    if !ui.is_open {
        return CraftingClickResult::None;
    }
    
    // Check crafting grid slots
    for i in 0..9 {
        let (x, y) = get_grid_slot_position(ui, i);
        let rect = UIRect {
            x,
            y,
            width: 48.0,
            height: 48.0,
        };
        
        if rect.contains(mouse_x, mouse_y) {
            return CraftingClickResult::GridSlot(i);
        }
    }
    
    // Check result slot
    let result_pos = get_result_slot_position(ui);
    let result_rect = UIRect {
        x: result_pos.x,
        y: result_pos.y,
        width: 48.0,
        height: 48.0,
    };
    
    if result_rect.contains(mouse_x, mouse_y) && ui.current_result.is_some() {
        return CraftingClickResult::ResultSlot;
    }
    
    CraftingClickResult::None
}

/// Get the position of a grid slot
/// Pure function - calculates slot position from UI data
fn get_grid_slot_position(ui: &CraftingTableUI, index: usize) -> (f32, f32) {
    let row = index / 3;
    let col = index % 3;
    
    let x = ui.position.x + 20.0 + col as f32 * 52.0;
    let y = ui.position.y + 20.0 + row as f32 * 52.0;
    
    (x, y)
}

/// Get the position of the result slot
/// Pure function - calculates result slot position from UI data
fn get_result_slot_position(ui: &CraftingTableUI) -> Vec2 {
    Vec2::new(
        ui.position.x + 220.0,
        ui.position.y + 74.0,
    )
}

/// Render the crafting table UI
/// Function - processes UI data for rendering output
pub fn render_crafting_table_ui(ui: &CraftingTableUI, ui_renderer: &mut UIRenderer, item_registry: &ItemRegistry) {
    if !ui.is_open {
        return;
    }
    
    // Draw background
    ui_renderer.draw_rect(UIRect {
        x: ui.position.x,
        y: ui.position.y,
        width: 320.0,
        height: 200.0,
    }, UIColor::new(0.0, 0.0, 0.0, 0.9));
    
    // Draw title
    ui_renderer.draw_text(
        "Crafting Table",
        ui.position.x + 160.0,
        ui.position.y + 10.0,
        16.0,
        UIColor::WHITE,
    );
    
    // Draw crafting grid
    for i in 0..9 {
        let (x, y) = get_grid_slot_position(ui, i);
        
        // Slot background
        ui_renderer.draw_rect(UIRect {
            x,
            y,
            width: 48.0,
            height: 48.0,
        }, UIColor::new(0.3, 0.3, 0.3, 0.8));
        
        // Slot border
        ui_renderer.draw_rect_outline(UIRect {
            x,
            y,
            width: 48.0,
            height: 48.0,
        }, UIColor::new(0.5, 0.5, 0.5, 1.0), 2.0);
        
        // Item in slot
        if let Some(item) = &ui.grid.slots[i] {
            render_crafting_item(ui_renderer, x, y, item, item_registry);
        }
    }
    
    // Draw arrow
    ui_renderer.draw_text(
        "→",
        ui.position.x + 180.0,
        ui.position.y + 90.0,
        24.0,
        UIColor::WHITE,
    );
    
    // Draw result slot
    let result_pos = get_result_slot_position(ui);
    
    // Result slot background
    ui_renderer.draw_rect(UIRect {
        x: result_pos.x,
        y: result_pos.y,
        width: 48.0,
        height: 48.0,
    }, UIColor::new(0.3, 0.3, 0.3, 0.8));
    
    // Result slot border
    ui_renderer.draw_rect_outline(UIRect {
        x: result_pos.x,
        y: result_pos.y,
        width: 48.0,
        height: 48.0,
    }, UIColor::new(0.7, 0.7, 0.2, 1.0), 2.0);
    
    // Result item
    if let Some(result) = &ui.current_result {
        render_crafting_item(ui_renderer, result_pos.x, result_pos.y, &result.output, item_registry);
    }
}

/// Render an item in a slot
/// Function - processes item data for rendering output
fn render_crafting_item(ui_renderer: &mut UIRenderer, x: f32, y: f32, item: &ItemStackData, item_registry: &ItemRegistry) {
    // Get item name
    let name = if let Some(item_type) = item_registry.get_item(ItemId(item.item_id)) {
        item_type.get_name()
    } else {
        format!("Item {}", item.item_id)
    };
    
    // Render item name (truncated)
    let display_name = if name.len() > 6 {
        format!("{}...", &name[..6])
    } else {
        name
    };
    
    ui_renderer.draw_text(
        &display_name,
        x + 24.0,
        y + 20.0,
        12.0,
        UIColor::WHITE,
    );
    
    // Render count if > 1
    if item.count > 1 {
        ui_renderer.draw_text(
            &item.count.to_string(),
            x + 36.0,
            y + 36.0,
            14.0,
            UIColor::WHITE,
        );
    }
}

/// Result of clicking on the crafting UI
#[derive(Debug, Clone, Copy)]
pub enum CraftingClickResult {
    None,
    GridSlot(usize),
    ResultSlot,
}