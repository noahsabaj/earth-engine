use crate::world::VoxelPos;
use crate::inventory::{ItemStackData, create_item_stack};
use crate::crafting::RecipeRegistry;
use crate::item::ItemId;
use std::collections::HashMap;

/// Trait for blocks that have additional data/behavior
pub trait BlockEntity: Send + Sync {
    /// Update the block entity (called each tick)
    fn update(&mut self, delta_time: f32);
    
    /// Get the position of this block entity
    fn get_position(&self) -> VoxelPos;
    
    /// Serialize the block entity data
    fn serialize(&self) -> BlockEntityData;
    
    /// Deserialize from data
    fn deserialize(data: BlockEntityData) -> Self where Self: Sized;
}

/// Serialized block entity data
#[derive(Debug, Clone)]
pub struct BlockEntityData {
    pub entity_type: String,
    pub position: VoxelPos,
    pub data: HashMap<String, serde_json::Value>,
}

/// A furnace block entity
#[derive(Debug, Clone)]
pub struct FurnaceBlockEntity {
    position: VoxelPos,
    /// Input slot (what's being smelted)
    input_slot: Option<ItemStackData>,
    /// Fuel slot
    fuel_slot: Option<ItemStackData>,
    /// Output slot
    output_slot: Option<ItemStackData>,
    /// Current smelting progress (0.0 - 1.0)
    smelt_progress: f32,
    /// Current fuel remaining (in seconds)
    fuel_remaining: f32,
    /// Total fuel time when fuel was added
    fuel_total: f32,
}

impl FurnaceBlockEntity {
    pub fn new(position: VoxelPos) -> Self {
        Self {
            position,
            input_slot: None,
            fuel_slot: None,
            output_slot: None,
            smelt_progress: 0.0,
            fuel_remaining: 0.0,
            fuel_total: 0.0,
        }
    }
    
    /// Get the input slot
    pub fn get_input(&self) -> Option<&ItemStackData> {
        self.input_slot.as_ref()
    }
    
    /// Get the fuel slot
    pub fn get_fuel(&self) -> Option<&ItemStackData> {
        self.fuel_slot.as_ref()
    }
    
    /// Get the output slot
    pub fn get_output(&self) -> Option<&ItemStackData> {
        self.output_slot.as_ref()
    }
    
    /// Set input item
    pub fn set_input(&mut self, item: Option<ItemStackData>) {
        if item.is_none() {
            self.smelt_progress = 0.0; // Reset progress if input removed
        }
        self.input_slot = item;
    }
    
    /// Set fuel item
    pub fn set_fuel(&mut self, item: Option<ItemStackData>) {
        self.fuel_slot = item;
    }
    
    /// Take from output slot
    pub fn take_output(&mut self) -> Option<ItemStackData> {
        self.output_slot.take()
    }
    
    /// Get smelting progress (0.0 - 1.0)
    pub fn get_smelt_progress(&self) -> f32 {
        self.smelt_progress
    }
    
    /// Get fuel progress (0.0 - 1.0)
    pub fn get_fuel_progress(&self) -> f32 {
        if self.fuel_total > 0.0 {
            self.fuel_remaining / self.fuel_total
        } else {
            0.0
        }
    }
    
    /// Check if furnace is active (has fuel and is smelting)
    pub fn is_active(&self) -> bool {
        self.fuel_remaining > 0.0 && self.input_slot.is_some()
    }
    
    /// Try to start smelting if possible
    fn try_start_smelting(&mut self, recipe_registry: &RecipeRegistry) {
        // Check if we have input and no current fuel
        if self.input_slot.is_some() && self.fuel_remaining <= 0.0 {
            // Try to consume fuel
            if let Some(fuel) = &mut self.fuel_slot {
                let fuel_time = get_fuel_burn_time(ItemId(fuel.item_id));
                if fuel_time > 0.0 {
                    // Consume one fuel
                    fuel.count -= 1;
                    if fuel.count == 0 {
                        self.fuel_slot = None;
                    }
                    
                    self.fuel_remaining = fuel_time;
                    self.fuel_total = fuel_time;
                }
            }
        }
    }
}

impl BlockEntity for FurnaceBlockEntity {
    fn update(&mut self, delta_time: f32) {
        // TODO: Get recipe registry from somewhere
        // For now, we'll just simulate smelting
        
        if self.fuel_remaining > 0.0 {
            self.fuel_remaining -= delta_time;
            
            if let Some(input) = &self.input_slot {
                // Simulate smelting (10 seconds per item)
                self.smelt_progress += delta_time / 10.0;
                
                if self.smelt_progress >= 1.0 {
                    // Smelting complete
                    self.smelt_progress = 0.0;
                    
                    // Create output (for now, just copy input)
                    let output = create_item_stack(ItemId(input.item_id), 1);
                    
                    // Add to output slot
                    if let Some(existing) = &mut self.output_slot {
                        if existing.item_id == output.item_id && existing.count < 64 {
                            existing.count += 1;
                        }
                    } else {
                        self.output_slot = Some(output);
                    }
                    
                    // Consume input
                    if let Some(input) = &mut self.input_slot {
                        input.count -= 1;
                        if input.count == 0 {
                            self.input_slot = None;
                        }
                    }
                }
            }
        } else {
            // No fuel, reset progress slowly
            if self.smelt_progress > 0.0 {
                self.smelt_progress = (self.smelt_progress - delta_time / 20.0).max(0.0);
            }
        }
    }
    
    fn get_position(&self) -> VoxelPos {
        self.position
    }
    
    fn serialize(&self) -> BlockEntityData {
        let mut data = HashMap::new();
        
        // Serialize slots
        if let Some(input) = &self.input_slot {
            data.insert("input_id".to_string(), serde_json::json!(input.item_id));
            data.insert("input_count".to_string(), serde_json::json!(input.count));
        }
        
        if let Some(fuel) = &self.fuel_slot {
            data.insert("fuel_id".to_string(), serde_json::json!(fuel.item_id));
            data.insert("fuel_count".to_string(), serde_json::json!(fuel.count));
        }
        
        if let Some(output) = &self.output_slot {
            data.insert("output_id".to_string(), serde_json::json!(output.item_id));
            data.insert("output_count".to_string(), serde_json::json!(output.count));
        }
        
        data.insert("smelt_progress".to_string(), serde_json::json!(self.smelt_progress));
        data.insert("fuel_remaining".to_string(), serde_json::json!(self.fuel_remaining));
        data.insert("fuel_total".to_string(), serde_json::json!(self.fuel_total));
        
        BlockEntityData {
            entity_type: "furnace".to_string(),
            position: self.position,
            data,
        }
    }
    
    fn deserialize(data: BlockEntityData) -> Self {
        let mut furnace = Self::new(data.position);
        
        // Deserialize slots
        if let (Some(id), Some(count)) = (
            data.data.get("input_id").and_then(|v| v.as_u64()),
            data.data.get("input_count").and_then(|v| v.as_u64()),
        ) {
            furnace.input_slot = Some(create_item_stack(ItemId(id as u32), count as u32));
        }
        
        if let (Some(id), Some(count)) = (
            data.data.get("fuel_id").and_then(|v| v.as_u64()),
            data.data.get("fuel_count").and_then(|v| v.as_u64()),
        ) {
            furnace.fuel_slot = Some(create_item_stack(ItemId(id as u32), count as u32));
        }
        
        if let (Some(id), Some(count)) = (
            data.data.get("output_id").and_then(|v| v.as_u64()),
            data.data.get("output_count").and_then(|v| v.as_u64()),
        ) {
            furnace.output_slot = Some(create_item_stack(ItemId(id as u32), count as u32));
        }
        
        if let Some(progress) = data.data.get("smelt_progress").and_then(|v| v.as_f64()) {
            furnace.smelt_progress = progress as f32;
        }
        
        if let Some(remaining) = data.data.get("fuel_remaining").and_then(|v| v.as_f64()) {
            furnace.fuel_remaining = remaining as f32;
        }
        
        if let Some(total) = data.data.get("fuel_total").and_then(|v| v.as_f64()) {
            furnace.fuel_total = total as f32;
        }
        
        furnace
    }
}

/// Get burn time for a fuel item (in seconds)
fn get_fuel_burn_time(item_id: ItemId) -> f32 {
    match item_id {
        ItemId::COAL => 80.0, // Coal burns for 80 seconds
        ItemId::WOOD_BLOCK => 15.0, // Wood burns for 15 seconds
        ItemId::PLANKS_BLOCK => 15.0, // Planks burn for 15 seconds
        ItemId::STICK => 5.0, // Sticks burn for 5 seconds
        _ => 0.0, // Not a fuel
    }
}

/// A chest block entity
#[derive(Debug, Clone)]
pub struct ChestBlockEntity {
    position: VoxelPos,
    /// Chest inventory (27 slots)
    slots: Vec<Option<ItemStackData>>,
}

impl ChestBlockEntity {
    pub fn new(position: VoxelPos) -> Self {
        Self {
            position,
            slots: vec![None; 27],
        }
    }
    
    /// Get a slot
    pub fn get_slot(&self, index: usize) -> Option<&ItemStackData> {
        self.slots.get(index).and_then(|s| s.as_ref())
    }
    
    /// Set a slot
    pub fn set_slot(&mut self, index: usize, item: Option<ItemStackData>) {
        if index < self.slots.len() {
            self.slots[index] = item;
        }
    }
    
    /// Find first empty slot
    pub fn find_empty_slot(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_none())
    }
    
    /// Try to add an item to the chest
    pub fn add_item(&mut self, mut item: ItemStackData) -> Option<ItemStackData> {
        // First try to merge with existing stacks
        for slot in &mut self.slots {
            if let Some(existing) = slot {
                if existing.item_id == item.item_id && existing.count < 64 {
                    let space = 64 - existing.count;
                    let to_add = item.count.min(space);
                    existing.count += to_add;
                    item.count -= to_add;
                    
                    if item.count == 0 {
                        return None;
                    }
                }
            }
        }
        
        // Then try to find empty slots
        while item.count > 0 {
            if let Some(empty_index) = self.find_empty_slot() {
                let to_place = item.count.min(64);
                self.slots[empty_index] = Some(create_item_stack(ItemId(item.item_id), to_place));
                item.count -= to_place;
            } else {
                break; // No more space
            }
        }
        
        if item.count > 0 {
            Some(item)
        } else {
            None
        }
    }
}

impl BlockEntity for ChestBlockEntity {
    fn update(&mut self, _delta_time: f32) {
        // Chests don't need updates
    }
    
    fn get_position(&self) -> VoxelPos {
        self.position
    }
    
    fn serialize(&self) -> BlockEntityData {
        let mut data = HashMap::new();
        
        for (i, slot) in self.slots.iter().enumerate() {
            if let Some(item) = slot {
                data.insert(format!("slot_{}_id", i), serde_json::json!(item.item_id));
                data.insert(format!("slot_{}_count", i), serde_json::json!(item.count));
            }
        }
        
        BlockEntityData {
            entity_type: "chest".to_string(),
            position: self.position,
            data,
        }
    }
    
    fn deserialize(data: BlockEntityData) -> Self {
        let mut chest = Self::new(data.position);
        
        for i in 0..27 {
            if let (Some(id), Some(count)) = (
                data.data.get(&format!("slot_{}_id", i)).and_then(|v| v.as_u64()),
                data.data.get(&format!("slot_{}_count", i)).and_then(|v| v.as_u64()),
            ) {
                chest.slots[i] = Some(create_item_stack(ItemId(id as u32), count as u32));
            }
        }
        
        chest
    }
}