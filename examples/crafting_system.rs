use earth_engine::{
    crafting::{RecipeRegistry, CraftingGrid, CraftingTableUI, Tool, ToolType, ToolMaterial, get_block_properties},
    inventory::{ItemStack, PlayerInventory},
    item::{ItemId, ItemRegistry},
    world::{BlockId, BlockDropHandler, MiningProgress, VoxelPos},
    renderer::ui::{UIRenderer, UIColor},
    ecs::EcsWorld,
};

fn main() {
    println!("=== Earth Engine Crafting System Demo ===\n");
    
    // Initialize registries
    let recipe_registry = RecipeRegistry::default();
    let item_registry = ItemRegistry::default();
    
    // Test 1: Recipe matching
    test_recipe_matching(&recipe_registry);
    
    // Test 2: Tool effectiveness
    test_tool_effectiveness();
    
    // Test 3: Crafting UI
    test_crafting_ui(&recipe_registry, &item_registry);
    
    // Test 4: Block drops
    test_block_drops(&item_registry);
    
    // Test 5: Furnace smelting
    test_furnace_smelting(&recipe_registry);
    
    println!("\n=== All tests completed! ===");
}

fn test_recipe_matching(registry: &RecipeRegistry) {
    println!("Test 1: Recipe Matching");
    println!("-----------------------");
    
    // Create a 3x3 crafting grid
    let mut grid = CraftingGrid::new(3, 3);
    
    // Test crafting sticks
    println!("\nCrafting sticks:");
    grid.set(1, 0, Some(ItemStack::new(ItemId::PLANKS_BLOCK, 1)));
    grid.set(1, 1, Some(ItemStack::new(ItemId::PLANKS_BLOCK, 1)));
    
    if let Some(recipe) = registry.find_recipe(&grid) {
        println!("  ✓ Found recipe: {}", recipe.id);
        if let earth_engine::crafting::RecipeType::Shaped(shaped) = &recipe.recipe_type {
            println!("  ✓ Result: {} x{}", shaped.result.item_id.0, shaped.result.count);
        }
    } else {
        println!("  ✗ No recipe found");
    }
    
    // Clear grid
    grid.clear();
    
    // Test crafting planks
    println!("\nCrafting planks:");
    grid.set(0, 0, Some(ItemStack::new(ItemId::WOOD_BLOCK, 1)));
    
    if let Some(recipe) = registry.find_recipe(&grid) {
        println!("  ✓ Found recipe: {}", recipe.id);
        if let earth_engine::crafting::RecipeType::Shapeless(shapeless) = &recipe.recipe_type {
            println!("  ✓ Result: {} x{}", shapeless.result.item_id.0, shapeless.result.count);
        }
    } else {
        println!("  ✗ No recipe found");
    }
    
    // Clear grid
    grid.clear();
    
    // Test crafting wooden pickaxe
    println!("\nCrafting wooden pickaxe:");
    // PPP
    //  S 
    //  S 
    grid.set(0, 0, Some(ItemStack::new(ItemId::PLANKS_BLOCK, 1)));
    grid.set(1, 0, Some(ItemStack::new(ItemId::PLANKS_BLOCK, 1)));
    grid.set(2, 0, Some(ItemStack::new(ItemId::PLANKS_BLOCK, 1)));
    grid.set(1, 1, Some(ItemStack::new(ItemId::STICK, 1)));
    grid.set(1, 2, Some(ItemStack::new(ItemId::STICK, 1)));
    
    if let Some(recipe) = registry.find_recipe(&grid) {
        println!("  ✓ Found recipe: {}", recipe.id);
        if let earth_engine::crafting::RecipeType::Shaped(shaped) = &recipe.recipe_type {
            println!("  ✓ Result: Wooden Pickaxe");
        }
    } else {
        println!("  ✗ No recipe found");
    }
}

fn test_tool_effectiveness() {
    println!("\n\nTest 2: Tool Effectiveness");
    println!("---------------------------");
    
    // Create different tools
    let wooden_pickaxe = Tool::new(ToolType::Pickaxe, ToolMaterial::Wood);
    let stone_pickaxe = Tool::new(ToolType::Pickaxe, ToolMaterial::Stone);
    let iron_pickaxe = Tool::new(ToolType::Pickaxe, ToolMaterial::Iron);
    
    // Test on different blocks
    let blocks = [
        (BlockId(3), "Stone"),
        (BlockId(10), "Iron Ore"),
        (BlockId(12), "Diamond Ore"),
    ];
    
    for (block_id, name) in &blocks {
        println!("\nMining {} with different tools:", name);
        let properties = get_block_properties(*block_id);
        
        // Test wooden pickaxe
        let effectiveness = wooden_pickaxe.get_effectiveness(*block_id, &properties);
        println!("  Wooden Pickaxe: speed={:.1}x, can_harvest={}", 
            effectiveness.speed_multiplier, effectiveness.can_harvest);
        
        // Test stone pickaxe
        let effectiveness = stone_pickaxe.get_effectiveness(*block_id, &properties);
        println!("  Stone Pickaxe:  speed={:.1}x, can_harvest={}", 
            effectiveness.speed_multiplier, effectiveness.can_harvest);
        
        // Test iron pickaxe
        let effectiveness = iron_pickaxe.get_effectiveness(*block_id, &properties);
        println!("  Iron Pickaxe:   speed={:.1}x, can_harvest={}", 
            effectiveness.speed_multiplier, effectiveness.can_harvest);
    }
    
    // Test durability
    println!("\nTool Durability:");
    let mut tool = Tool::new(ToolType::Pickaxe, ToolMaterial::Wood);
    println!("  Wooden Pickaxe durability: {}/{}", 
        tool.durability.current, tool.durability.max);
    
    // Use the tool 10 times
    for _ in 0..10 {
        tool.durability.use_tool();
    }
    println!("  After 10 uses: {}/{} ({:.0}%)", 
        tool.durability.current, tool.durability.max, 
        tool.durability.percentage() * 100.0);
}

fn test_crafting_ui(registry: &RecipeRegistry, item_registry: &ItemRegistry) {
    println!("\n\nTest 3: Crafting UI");
    println!("--------------------");
    
    // Create crafting table UI
    let mut ui = CraftingTableUI::new(1280.0, 720.0);
    ui.open();
    
    println!("  ✓ Crafting table UI created and opened");
    
    // Simulate placing items in the grid
    ui.place_item(0, ItemStack::new(ItemId::PLANKS_BLOCK, 1));
    ui.place_item(1, ItemStack::new(ItemId::PLANKS_BLOCK, 1));
    ui.place_item(3, ItemStack::new(ItemId::PLANKS_BLOCK, 1));
    ui.place_item(4, ItemStack::new(ItemId::PLANKS_BLOCK, 1));
    
    // Update the result
    ui.update_result(registry);
    
    // Try to craft
    if let Some(result) = ui.craft() {
        println!("  ✓ Crafted: {} x{}", 
            item_registry.get_item(result.item_id).map(|i| i.get_name()).unwrap_or_default(),
            result.count
        );
    } else {
        println!("  ✗ No valid recipe");
    }
    
    ui.close();
    println!("  ✓ Crafting table UI closed");
}

fn test_block_drops(item_registry: &ItemRegistry) {
    println!("\n\nTest 4: Block Drops");
    println!("--------------------");
    
    // Create an ECS world for item entities
    let mut ecs_world = EcsWorld::new();
    
    // Test breaking different blocks
    let blocks_to_break = [
        (BlockId(3), "Stone", Some(Tool::new(ToolType::Pickaxe, ToolMaterial::Wood))),
        (BlockId(10), "Iron Ore", Some(Tool::new(ToolType::Pickaxe, ToolMaterial::Stone))),
        (BlockId(4), "Wood", None), // Break by hand
    ];
    
    for (block_id, name, tool) in blocks_to_break {
        println!("\nBreaking {} with {:?}:", name, 
            tool.as_ref().map(|t| format!("{:?} {:?}", t.material, t.tool_type))
                .unwrap_or_else(|| "hand".to_string())
        );
        
        // Calculate mining time
        let mining_time = BlockDropHandler::calculate_mining_time(block_id, tool.as_ref());
        
        if mining_time.is_finite() {
            println!("  Mining time: {:.1}s", mining_time);
            
            // Simulate mining progress
            let mut progress = MiningProgress::new(VoxelPos { x: 0, y: 0, z: 0 }, mining_time);
            
            // Update for half the time
            progress.update(mining_time / 2.0);
            println!("  Progress at halfway: {:.0}%", progress.progress * 100.0);
            
            // Complete mining
            let completed = progress.update(mining_time / 2.0);
            println!("  Mining completed: {}", completed);
        } else {
            println!("  Cannot break this block with current tool!");
        }
    }
}

fn test_furnace_smelting(registry: &RecipeRegistry) {
    println!("\n\nTest 5: Furnace Smelting");
    println!("-------------------------");
    
    // Find smelting recipes
    let smelting_inputs = [
        (ItemId::COBBLESTONE_BLOCK, "Cobblestone"),
        (ItemId::IRON_ORE_BLOCK, "Iron Ore"),
    ];
    
    for (input_id, name) in &smelting_inputs {
        if let Some(recipe) = registry.find_smelting_recipe(*input_id) {
            if let earth_engine::crafting::RecipeType::Smelting(smelting) = &recipe.recipe_type {
                println!("\nSmelting {}:", name);
                println!("  ✓ Output: {} x{}", 
                    smelting.output.item_id.0, 
                    smelting.output.count
                );
                println!("  ✓ Time: {:.1}s", smelting.smelt_time);
                println!("  ✓ Experience: {:.1}", smelting.experience);
            }
        } else {
            println!("\n✗ No smelting recipe for {}", name);
        }
    }
    
    // Test furnace block entity
    use earth_engine::world::{FurnaceBlockEntity, VoxelPos};
    
    let mut furnace = FurnaceBlockEntity::new(VoxelPos { x: 0, y: 64, z: 0 });
    
    println!("\nFurnace simulation:");
    furnace.set_input(Some(ItemStack::new(ItemId::IRON_ORE_BLOCK, 1)));
    furnace.set_fuel(Some(ItemStack::new(ItemId::COAL, 1)));
    
    println!("  Input: Iron Ore x1");
    println!("  Fuel: Coal x1");
    
    // Simulate 5 seconds of smelting
    for i in 0..5 {
        furnace.update(1.0);
        println!("  {}s: Progress={:.0}%, Fuel={:.0}%", 
            i + 1,
            furnace.get_smelt_progress() * 100.0,
            furnace.get_fuel_progress() * 100.0
        );
    }
}