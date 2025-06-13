use crate::crafting::{RecipeRegistry, CraftingGrid};
use crate::inventory::{ItemStackData, create_item_stack};
use crate::item::ItemRegistry;
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

impl CraftingTableUI {
    /// Create a new crafting table UI
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        let grid_size = 3;
        let ui_width = (grid_size + 2) as f32 * 52.0 + 100.0; // Grid + result + padding
        let ui_height = grid_size as f32 * 52.0 + 200.0; // Grid + inventory + padding
        
        Self {
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
    pub fn open(&mut self) {
        self.is_open = true;
    }
    
    /// Close the crafting table UI
    pub fn close(&mut self) {
        self.is_open = false;
        // Return items to inventory when closing
        // In a real implementation, this would drop items or return to inventory
        self.grid.clear();
        self.current_result = None;
    }
    
    /// Check if the UI is open
    pub fn is_open(&self) -> bool {
        self.is_open
    }
    
    /// Update the crafting result based on current grid
    pub fn update_result(&mut self, recipe_registry: &RecipeRegistry) {
        if let Some(recipe) = recipe_registry.find_recipe(&self.grid) {
            match &recipe.recipe_type {
                crate::crafting::RecipeType::Shaped(shaped) => {
                    self.current_result = Some(CraftingResult {
                        output: shaped.result.clone(),
                        consumes: self.calculate_consumes(),
                    });
                }
                crate::crafting::RecipeType::Shapeless(shapeless) => {
                    self.current_result = Some(CraftingResult {
                        output: shapeless.result.clone(),
                        consumes: self.calculate_consumes(),
                    });
                }
                _ => {
                    self.current_result = None;
                }
            }
        } else {
            self.current_result = None;
        }
    }
    
    /// Calculate what items need to be consumed for crafting
    fn calculate_consumes(&self) -> Vec<(usize, u32)> {
        let mut consumes = Vec::new();
        
        for (index, slot) in self.grid.slots.iter().enumerate() {
            if slot.is_some() {
                consumes.push((index, 1)); // Consume 1 of each item
            }
        }
        
        consumes
    }
    
    /// Place an item in the crafting grid
    pub fn place_item(&mut self, grid_index: usize, item: ItemStackData) -> Option<ItemStackData> {
        if grid_index < self.grid.slots.len() {
            let old = self.grid.slots[grid_index].take();
            self.grid.slots[grid_index] = Some(item);
            old
        } else {
            Some(item) // Invalid index, return the item
        }
    }
    
    /// Take an item from the crafting grid
    pub fn take_item(&mut self, grid_index: usize) -> Option<ItemStackData> {
        if grid_index < self.grid.slots.len() {
            self.grid.slots[grid_index].take()
        } else {
            None
        }
    }
    
    /// Craft the current result
    pub fn craft(&mut self) -> Option<ItemStackData> {
        if let Some(result) = &self.current_result {
            // Consume items from grid
            for &(index, count) in &result.consumes {
                if let Some(item) = &mut self.grid.slots[index] {
                    if item.count > count {
                        item.count -= count;
                    } else {
                        self.grid.slots[index] = None;
                    }
                }
            }
            
            // Clear the result
            let output = result.output.clone();
            self.current_result = None;
            
            Some(output)
        } else {
            None
        }
    }
    
    /// Handle mouse click on the UI
    pub fn handle_click(&mut self, mouse_x: f32, mouse_y: f32) -> CraftingClickResult {
        if !self.is_open {
            return CraftingClickResult::None;
        }
        
        // Check crafting grid slots
        for i in 0..9 {
            let (x, y) = self.get_grid_slot_position(i);
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
        let result_pos = self.get_result_slot_position();
        let result_rect = UIRect {
            x: result_pos.x,
            y: result_pos.y,
            width: 48.0,
            height: 48.0,
        };
        
        if result_rect.contains(mouse_x, mouse_y) && self.current_result.is_some() {
            return CraftingClickResult::ResultSlot;
        }
        
        CraftingClickResult::None
    }
    
    /// Get the position of a grid slot
    fn get_grid_slot_position(&self, index: usize) -> (f32, f32) {
        let row = index / 3;
        let col = index % 3;
        
        let x = self.position.x + 20.0 + col as f32 * 52.0;
        let y = self.position.y + 20.0 + row as f32 * 52.0;
        
        (x, y)
    }
    
    /// Get the position of the result slot
    fn get_result_slot_position(&self) -> Vec2 {
        Vec2::new(
            self.position.x + 220.0,
            self.position.y + 74.0,
        )
    }
    
    /// Render the crafting table UI
    pub fn render(&self, ui_renderer: &mut UIRenderer, item_registry: &ItemRegistry) {
        if !self.is_open {
            return;
        }
        
        // Draw background
        ui_renderer.draw_rect(UIRect {
            x: self.position.x,
            y: self.position.y,
            width: 320.0,
            height: 200.0,
        }, UIColor::new(0.0, 0.0, 0.0, 0.9));
        
        // Draw title
        ui_renderer.draw_text(
            "Crafting Table",
            self.position.x + 160.0,
            self.position.y + 10.0,
            16.0,
            UIColor::WHITE,
        );
        
        // Draw crafting grid
        for i in 0..9 {
            let (x, y) = self.get_grid_slot_position(i);
            
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
            if let Some(item) = &self.grid.slots[i] {
                self.render_item(ui_renderer, x, y, item, item_registry);
            }
        }
        
        // Draw arrow
        ui_renderer.draw_text(
            "→",
            self.position.x + 180.0,
            self.position.y + 90.0,
            24.0,
            UIColor::WHITE,
        );
        
        // Draw result slot
        let result_pos = self.get_result_slot_position();
        
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
        if let Some(result) = &self.current_result {
            self.render_item(ui_renderer, result_pos.x, result_pos.y, &result.output, item_registry);
        }
    }
    
    /// Render an item in a slot
    fn render_item(&self, ui_renderer: &mut UIRenderer, x: f32, y: f32, item: &ItemStackData, item_registry: &ItemRegistry) {
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
}

/// Result of clicking on the crafting UI
#[derive(Debug, Clone, Copy)]
pub enum CraftingClickResult {
    None,
    GridSlot(usize),
    ResultSlot,
}