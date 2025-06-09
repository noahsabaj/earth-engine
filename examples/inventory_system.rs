use earth_engine::{
    BlockId, Camera, Engine, EngineConfig, Game, GameContext, 
    ecs::{EcsWorld, Entity, System},
    ecs::components::{Transform, Physics, ItemComponent},
    ecs::systems::{ItemPickupSystem, ItemPhysicsSystem, InventorySystem, create_item_entity},
    inventory::{PlayerInventory, ItemStack, ItemDropHandler},
    ui::{InventoryUI, InventoryInputHandler},
    KeyCode,
};
use glam::{Vec3, Quat};

/// Example game demonstrating the inventory system
struct InventoryDemo {
    ecs_world: EcsWorld,
    player_entity: Entity,
    inventory_system: InventorySystem,
    pickup_system: ItemPickupSystem,
    physics_system: ItemPhysicsSystem,
    inventory_ui: InventoryUI,
    input_handler: InventoryInputHandler,
    camera: Camera,
}

impl InventoryDemo {
    fn new() -> Self {
        let mut ecs_world = EcsWorld::new();
        
        // Create player entity
        let player_entity = ecs_world.create_entity();
        ecs_world.add_component(player_entity, Transform {
            position: Vec3::new(0.0, 5.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        });
        
        // Create some test items on the ground
        for i in 0..5 {
            let position = Vec3::new(i as f32 * 2.0 - 4.0, 1.0, 5.0);
            let block_id = match i {
                0 => BlockId::Dirt,
                1 => BlockId::Stone,
                2 => BlockId::Wood,
                3 => BlockId::Sand,
                _ => BlockId::Grass,
            };
            
            create_item_entity(
                &mut ecs_world,
                position,
                Vec3::ZERO,
                block_id,
                (i + 1) * 10, // Different stack sizes
            );
        }
        
        // Initialize systems
        let inventory_system = InventorySystem::new(player_entity);
        let pickup_system = ItemPickupSystem::new(player_entity);
        let physics_system = ItemPhysicsSystem::new();
        
        // Initialize UI
        let inventory_ui = InventoryUI::new(1280.0, 720.0);
        let input_handler = InventoryInputHandler::new();
        
        // Initialize camera
        let camera = Camera::new(
            Vec3::new(0.0, 5.0, -10.0),
            0.0,
            0.0,
            90.0,
            1280.0 / 720.0,
        );
        
        Self {
            ecs_world,
            player_entity,
            inventory_system,
            pickup_system,
            physics_system,
            inventory_ui,
            input_handler,
            camera,
        }
    }
    
    fn get_player_forward(&self) -> Vec3 {
        // Calculate forward vector from camera yaw
        let yaw_rad = self.camera.yaw.to_radians();
        Vec3::new(
            yaw_rad.sin(),
            0.0,
            yaw_rad.cos(),
        ).normalize()
    }
}

impl Game for InventoryDemo {
    fn init(&mut self, ctx: &mut GameContext) {
        println!("Inventory System Demo");
        println!("Controls:");
        println!("  WASD - Move");
        println!("  Mouse - Look around");
        println!("  E - Open/close inventory");
        println!("  Q - Drop item (Ctrl+Q to drop stack)");
        println!("  1-9 - Select hotbar slot");
        println!("  Scroll - Change hotbar selection");
        println!("  Walk near items to pick them up!");
        
        // Add some items to inventory for testing
        let inventory = self.inventory_system.get_inventory_mut();
        inventory.add_item(ItemStack::new(BlockId::Stone, 64));
        inventory.add_item(ItemStack::new(BlockId::Wood, 32));
        inventory.add_item(ItemStack::new(BlockId::Dirt, 16));
    }
    
    fn update(&mut self, ctx: &mut GameContext, delta_time: f32) {
        // Update player position from camera
        if let Some(transform) = self.ecs_world.get_component_mut::<Transform>(self.player_entity) {
            transform.position = self.camera.position;
        }
        
        // Update ECS systems
        self.physics_system.update(&mut self.ecs_world, delta_time);
        self.pickup_system.update(&mut self.ecs_world, delta_time);
        
        // Handle inventory input
        let player_pos = self.camera.position;
        let player_forward = self.get_player_forward();
        
        self.input_handler.update(
            &mut self.inventory_ui,
            self.inventory_system.get_inventory_mut(),
            &mut self.ecs_world,
            player_pos,
            player_forward,
        );
        
        // Update camera movement
        let speed = 5.0 * delta_time;
        if ctx.is_key_pressed(KeyCode::W) {
            self.camera.position += self.get_player_forward() * speed;
        }
        if ctx.is_key_pressed(KeyCode::S) {
            self.camera.position -= self.get_player_forward() * speed;
        }
        if ctx.is_key_pressed(KeyCode::A) {
            let right = self.get_player_forward().cross(Vec3::Y);
            self.camera.position -= right * speed;
        }
        if ctx.is_key_pressed(KeyCode::D) {
            let right = self.get_player_forward().cross(Vec3::Y);
            self.camera.position += right * speed;
        }
    }
    
    fn render(&mut self, ctx: &mut GameContext) {
        // In a real implementation, this would:
        // 1. Render the world
        // 2. Render item entities
        // 3. Render the inventory UI
        
        // For now, just print inventory state occasionally
        static mut FRAME_COUNT: u32 = 0;
        unsafe {
            FRAME_COUNT += 1;
            if FRAME_COUNT % 60 == 0 {
                let inventory = self.inventory_system.get_inventory();
                println!("\nInventory State:");
                println!("Selected slot: {}", inventory.selected_hotbar_index());
                
                for (i, slot) in inventory.hotbar_slots().iter().enumerate() {
                    if let Some(item) = slot.get_item() {
                        println!("  Hotbar {}: {:?} x{}", i + 1, item.block_id, item.count);
                    }
                }
                
                let total_items: u32 = (0..36)
                    .filter_map(|i| inventory.get_slot(i))
                    .filter_map(|slot| slot.get_item())
                    .map(|item| item.count)
                    .sum();
                println!("Total items in inventory: {}", total_items);
            }
        }
    }
    
    fn handle_key_press(&mut self, ctx: &mut GameContext, key: KeyCode) {
        self.input_handler.handle_key_press(key);
    }
    
    fn handle_key_release(&mut self, ctx: &mut GameContext, key: KeyCode) {
        self.input_handler.handle_key_release(key);
    }
    
    fn handle_mouse_motion(&mut self, ctx: &mut GameContext, delta_x: f64, delta_y: f64) {
        // Update camera look
        let sensitivity = 0.1;
        self.camera.yaw += delta_x as f32 * sensitivity;
        self.camera.pitch += delta_y as f32 * sensitivity;
        self.camera.pitch = self.camera.pitch.clamp(-89.0, 89.0);
    }
    
    fn handle_mouse_wheel(&mut self, ctx: &mut GameContext, delta: f32) {
        self.input_handler.handle_scroll(delta, self.inventory_system.get_inventory_mut());
    }
}

fn main() {
    let config = EngineConfig {
        window_title: "Earth Engine - Inventory System Demo".to_string(),
        ..Default::default()
    };
    
    let engine = Engine::new(config);
    let game = InventoryDemo::new();
    
    if let Err(e) = engine.run(game) {
        eprintln!("Error running inventory demo: {}", e);
    }
}