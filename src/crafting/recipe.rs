use crate::{
    inventory::{ItemStackData, create_item_stack}, 
    item::ItemId
};

// Type alias for compatibility
type ItemStack = ItemStackData;
use std::collections::HashMap;

/// Type of crafting recipe
#[derive(Debug, Clone, PartialEq)]
pub enum RecipeType {
    Shaped(ShapedRecipe),
    Shapeless(ShapelessRecipe),
    Smelting(SmeltingRecipe),
}

/// A shaped crafting recipe with specific pattern
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedRecipe {
    /// Pattern of the recipe (up to 3x3)
    /// Each string represents a row, each char represents an item
    pub pattern: Vec<String>,
    /// Mapping from pattern characters to item IDs
    pub key: HashMap<char, ItemId>,
    /// Result of the recipe
    pub result: ItemStackData,
}

/// A shapeless crafting recipe
#[derive(Debug, Clone, PartialEq)]
pub struct ShapelessRecipe {
    /// Required ingredients
    pub ingredients: Vec<ItemId>,
    /// Result of the recipe
    pub result: ItemStackData,
}

/// A smelting recipe for furnaces
#[derive(Debug, Clone, PartialEq)]
pub struct SmeltingRecipe {
    /// Input item
    pub input: ItemId,
    /// Output item
    pub output: ItemStackData,
    /// Smelting time in seconds
    pub smelt_time: f32,
    /// Experience gained
    pub experience: f32,
}

/// A crafting recipe
#[derive(Debug, Clone)]
pub struct Recipe {
    pub id: String,
    pub recipe_type: RecipeType,
}

impl Recipe {
    pub fn shaped(id: String, pattern: Vec<&str>, key: HashMap<char, ItemId>, result: ItemStack) -> Self {
        Self {
            id,
            recipe_type: RecipeType::Shaped(ShapedRecipe {
                pattern: pattern.into_iter().map(|s| s.to_string()).collect(),
                key,
                result,
            }),
        }
    }
    
    pub fn shapeless(id: String, ingredients: Vec<ItemId>, result: ItemStack) -> Self {
        Self {
            id,
            recipe_type: RecipeType::Shapeless(ShapelessRecipe {
                ingredients,
                result,
            }),
        }
    }
    
    pub fn smelting(id: String, input: ItemId, output: ItemStack, smelt_time: f32, experience: f32) -> Self {
        Self {
            id,
            recipe_type: RecipeType::Smelting(SmeltingRecipe {
                input,
                output,
                smelt_time,
                experience,
            }),
        }
    }
}

/// Represents a crafting grid (2x2 or 3x3)
#[derive(Debug, Clone)]
pub struct CraftingGrid {
    pub slots: Vec<Option<ItemStack>>,
    pub width: usize,
    pub height: usize,
}

impl CraftingGrid {
    /// Create a new crafting grid
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            slots: vec![None; width * height],
            width,
            height,
        }
    }
    
    /// Get item at position
    pub fn get(&self, x: usize, y: usize) -> Option<&ItemStack> {
        if x < self.width && y < self.height {
            self.slots[y * self.width + x].as_ref()
        } else {
            None
        }
    }
    
    
    /// Check if grid is empty
    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|slot| slot.is_none())
    }
    
    /// Get the bounds of the actual recipe in the grid
    fn get_recipe_bounds(&self) -> Option<(usize, usize, usize, usize)> {
        let mut min_x = self.width;
        let mut min_y = self.height;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut found_item = false;
        
        for y in 0..self.height {
            for x in 0..self.width {
                if self.get(x, y).is_some() {
                    found_item = true;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }
        
        if found_item {
            Some((min_x, min_y, max_x, max_y))
        } else {
            None
        }
    }
}

/// Registry for all crafting recipes
pub struct RecipeRegistry {
    recipes: Vec<Recipe>,
    shaped_recipes: Vec<usize>,
    shapeless_recipes: Vec<usize>,
    smelting_recipes: HashMap<ItemId, usize>,
}

impl RecipeRegistry {
    pub fn new() -> Self {
        Self {
            recipes: Vec::new(),
            shaped_recipes: Vec::new(),
            shapeless_recipes: Vec::new(),
            smelting_recipes: HashMap::new(),
        }
    }
    
    
    /// Find a matching recipe for the crafting grid
    pub fn find_recipe(&self, grid: &CraftingGrid) -> Option<&Recipe> {
        // First try shaped recipes
        for &index in &self.shaped_recipes {
            let recipe = &self.recipes[index];
            if let RecipeType::Shaped(shaped) = &recipe.recipe_type {
                if self.matches_shaped(grid, shaped) {
                    return Some(recipe);
                }
            }
        }
        
        // Then try shapeless recipes
        for &index in &self.shapeless_recipes {
            let recipe = &self.recipes[index];
            if let RecipeType::Shapeless(shapeless) = &recipe.recipe_type {
                if self.matches_shapeless(grid, shapeless) {
                    return Some(recipe);
                }
            }
        }
        
        None
    }
    
    /// Find a smelting recipe for an input
    pub fn find_smelting_recipe(&self, input: ItemId) -> Option<&Recipe> {
        self.smelting_recipes.get(&input)
            .and_then(|&index| self.recipes.get(index))
    }
    
    /// Check if a shaped recipe matches the grid
    fn matches_shaped(&self, grid: &CraftingGrid, recipe: &ShapedRecipe) -> bool {
        let bounds = match grid.get_recipe_bounds() {
            Some(b) => b,
            None => return false,
        };
        
        let (min_x, min_y, max_x, max_y) = bounds;
        let grid_width = max_x - min_x + 1;
        let grid_height = max_y - min_y + 1;
        
        // Check if recipe fits
        if recipe.pattern.len() != grid_height {
            return false;
        }
        
        for (y, row) in recipe.pattern.iter().enumerate() {
            if row.len() != grid_width {
                return false;
            }
            
            for (x, ch) in row.chars().enumerate() {
                let grid_item = grid.get(min_x + x, min_y + y);
                
                if ch == ' ' {
                    // Empty space in pattern
                    if grid_item.is_some() {
                        return false;
                    }
                } else {
                    // Check if item matches
                    let expected_item = match recipe.key.get(&ch) {
                        Some(block) => block,
                        None => return false, // Invalid recipe
                    };
                    
                    match grid_item {
                        Some(item) if ItemId(item.item_id) == *expected_item && item.count >= 1 => {
                            // Matches
                        }
                        _ => return false,
                    }
                }
            }
        }
        
        true
    }
    
    /// Check if a shapeless recipe matches the grid
    fn matches_shapeless(&self, grid: &CraftingGrid, recipe: &ShapelessRecipe) -> bool {
        let mut required = recipe.ingredients.clone();
        let mut found_count = 0;
        
        // Check each slot in the grid
        for slot in &grid.slots {
            if let Some(item) = slot {
                if let Some(pos) = required.iter().position(|&item_id| item_id == ItemId(item.item_id)) {
                    required.remove(pos);
                    found_count += 1;
                } else {
                    // Item not in recipe
                    return false;
                }
            }
        }
        
        // All required items must be found
        required.is_empty() && found_count == recipe.ingredients.len()
    }
    
}

/// Set item at position in crafting grid
/// Function - transforms grid data by setting item
pub fn set_item_in_grid(grid: &mut CraftingGrid, x: usize, y: usize, item: Option<ItemStack>) {
    if x < grid.width && y < grid.height {
        grid.slots[y * grid.width + x] = item;
    }
}

/// Clear the crafting grid
/// Function - transforms grid data by clearing all slots
pub fn clear_crafting_grid(grid: &mut CraftingGrid) {
    grid.slots.fill(None);
}

/// Register a new recipe in the registry
/// Function - transforms registry data by adding recipe
pub fn register_recipe(registry: &mut RecipeRegistry, recipe: Recipe) {
    let index = registry.recipes.len();
    
    match &recipe.recipe_type {
        RecipeType::Shaped(_) => registry.shaped_recipes.push(index),
        RecipeType::Shapeless(_) => registry.shapeless_recipes.push(index),
        RecipeType::Smelting(smelting) => {
            registry.smelting_recipes.insert(smelting.input, index);
        }
    }
    
    registry.recipes.push(recipe);
}

/// Initialize registry with default Minecraft-like recipes
/// Function - transforms registry data by adding all default recipes
pub fn init_default_recipes_in_registry(registry: &mut RecipeRegistry) {
    use crate::item::ItemId;
    
    // Planks from logs
    register_recipe(registry, Recipe::shapeless(
        "planks_from_wood".to_string(),
        vec![ItemId::WOOD_BLOCK],
        create_item_stack(ItemId::PLANKS_BLOCK, 4),
    ));
    
    // Sticks from planks
    register_recipe(registry, Recipe::shaped(
        "sticks".to_string(),
        vec!["P", "P"],
        [('P', ItemId::PLANKS_BLOCK)].into_iter().collect(),
        create_item_stack(ItemId::STICK, 4),
    ));
    
    // Wooden pickaxe
    register_recipe(registry, Recipe::shaped(
        "wooden_pickaxe".to_string(),
        vec!["PPP", " S ", " S "],
        [('P', ItemId::PLANKS_BLOCK), ('S', ItemId::STICK)].into_iter().collect(),
        create_item_stack(ItemId::WOODEN_PICKAXE, 1),
    ));
    
    // Stone pickaxe
    register_recipe(registry, Recipe::shaped(
        "stone_pickaxe".to_string(),
        vec!["SSS", " T ", " T "],
        [('S', ItemId::COBBLESTONE_BLOCK), ('T', ItemId::STICK)].into_iter().collect(),
        create_item_stack(ItemId::STONE_PICKAXE, 1),
    ));
    
    // Iron pickaxe
    register_recipe(registry, Recipe::shaped(
        "iron_pickaxe".to_string(),
        vec!["III", " S ", " S "],
        [('I', ItemId::IRON_INGOT), ('S', ItemId::STICK)].into_iter().collect(),
        create_item_stack(ItemId::IRON_PICKAXE, 1),
    ));
    
    // Wooden axe
    register_recipe(registry, Recipe::shaped(
        "wooden_axe".to_string(),
        vec!["PP ", "PS ", " S "],
        [('P', ItemId::PLANKS_BLOCK), ('S', ItemId::STICK)].into_iter().collect(),
        create_item_stack(ItemId::WOODEN_AXE, 1),
    ));
    
    // Stone from cobblestone (smelting)
    register_recipe(registry, Recipe::smelting(
        "stone_from_cobblestone".to_string(),
        ItemId::COBBLESTONE_BLOCK,
        create_item_stack(ItemId::STONE_BLOCK, 1),
        10.0, // 10 seconds
        0.1, // 0.1 experience
    ));
    
    // Iron ingot from iron ore (smelting)
    register_recipe(registry, Recipe::smelting(
        "iron_from_ore".to_string(),
        ItemId::IRON_ORE_BLOCK,
        create_item_stack(ItemId::IRON_INGOT, 1),
        10.0,
        0.7,
    ));
    
    // Torch
    register_recipe(registry, Recipe::shaped(
        "torch".to_string(),
        vec!["C", "S"],
        [('C', ItemId::COAL), ('S', ItemId::STICK)].into_iter().collect(),
        create_item_stack(ItemId::TORCH_BLOCK, 4),
    ));
    
    // Crafting table
    register_recipe(registry, Recipe::shaped(
        "crafting_table".to_string(),
        vec!["PP", "PP"],
        [('P', ItemId::PLANKS_BLOCK)].into_iter().collect(),
        create_item_stack(ItemId::CRAFTING_TABLE_BLOCK, 1),
    ));
    
    // Furnace
    register_recipe(registry, Recipe::shaped(
        "furnace".to_string(),
        vec!["CCC", "C C", "CCC"],
        [('C', ItemId::COBBLESTONE_BLOCK)].into_iter().collect(),
        create_item_stack(ItemId::FURNACE_BLOCK, 1),
    ));
    
    // Chest
    register_recipe(registry, Recipe::shaped(
        "chest".to_string(),
        vec!["PPP", "P P", "PPP"],
        [('P', ItemId::PLANKS_BLOCK)].into_iter().collect(),
        create_item_stack(ItemId::CHEST_BLOCK, 1),
    ));
}

/// Create a recipe registry with default recipes
/// Pure function - returns registry data with defaults pre-loaded
pub fn create_default_recipe_registry() -> RecipeRegistry {
    let mut registry = RecipeRegistry::new();
    init_default_recipes_in_registry(&mut registry);
    registry
}

impl Default for RecipeRegistry {
    fn default() -> Self {
        create_default_recipe_registry()
    }
}