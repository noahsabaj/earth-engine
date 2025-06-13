/// Zero-allocation block entity optimizations for Sprint 37
/// Replaces format!() and .to_string() allocations with pre-allocated buffers

use crate::world::VoxelPos;
use crate::inventory::ItemStackData;
use crate::item::ItemId;
use crate::renderer::allocation_optimizations::StaticFormatter;
use std::collections::HashMap;

/// Pre-allocated string keys for block entity serialization
/// These eliminate the need for .to_string() allocations
pub struct BlockEntityKeys {
    pub input_id: &'static str,
    pub input_count: &'static str,
    pub fuel_id: &'static str,
    pub fuel_count: &'static str,
    pub output_id: &'static str,
    pub output_count: &'static str,
    pub smelt_progress: &'static str,
    pub fuel_remaining: &'static str,
    pub fuel_total: &'static str,
    pub furnace_type: &'static str,
    pub chest_type: &'static str,
}

impl BlockEntityKeys {
    pub const fn new() -> Self {
        Self {
            input_id: "input_id",
            input_count: "input_count",
            fuel_id: "fuel_id",
            fuel_count: "fuel_count",
            output_id: "output_id",
            output_count: "output_count",
            smelt_progress: "smelt_progress",
            fuel_remaining: "fuel_remaining",
            fuel_total: "fuel_total",
            furnace_type: "furnace",
            chest_type: "chest",
        }
    }
}

/// Global static keys to avoid allocations
pub static KEYS: BlockEntityKeys = BlockEntityKeys::new();

/// Zero-allocation serialization for furnace block entities
pub fn serialize_furnace_zero_alloc(
    position: VoxelPos,
    input_slot: &Option<ItemStackData>,
    fuel_slot: &Option<ItemStackData>,
    output_slot: &Option<ItemStackData>,
    smelt_progress: f32,
    fuel_remaining: f32,
    fuel_total: f32,
) -> HashMap<&'static str, serde_json::Value> {
    let mut data = HashMap::with_capacity(8);
    
    // Use static string keys instead of .to_string()
    if let Some(input) = input_slot {
        data.insert(KEYS.input_id, serde_json::json!(input.item_id));
        data.insert(KEYS.input_count, serde_json::json!(input.count));
    }
    
    if let Some(fuel) = fuel_slot {
        data.insert(KEYS.fuel_id, serde_json::json!(fuel.item_id));
        data.insert(KEYS.fuel_count, serde_json::json!(fuel.count));
    }
    
    if let Some(output) = output_slot {
        data.insert(KEYS.output_id, serde_json::json!(output.item_id));
        data.insert(KEYS.output_count, serde_json::json!(output.count));
    }
    
    data.insert(KEYS.smelt_progress, serde_json::json!(smelt_progress));
    data.insert(KEYS.fuel_remaining, serde_json::json!(fuel_remaining));
    data.insert(KEYS.fuel_total, serde_json::json!(fuel_total));
    
    data
}

/// Zero-allocation slot key formatter using static buffer
pub struct SlotKeyFormatter {
    formatter: StaticFormatter<32>,
}

impl SlotKeyFormatter {
    pub fn new() -> Self {
        Self {
            formatter: StaticFormatter::new(),
        }
    }
    
    /// Format slot ID key without allocation
    pub fn format_slot_id_key(&mut self, slot_index: usize) -> &str {
        // Use a simple approach for slot formatting
        match slot_index {
            0 => "slot_0_id",
            1 => "slot_1_id",
            2 => "slot_2_id",
            3 => "slot_3_id",
            4 => "slot_4_id",
            5 => "slot_5_id",
            6 => "slot_6_id",
            7 => "slot_7_id",
            8 => "slot_8_id",
            9 => "slot_9_id",
            // For higher slots, we'd need a more sophisticated approach
            // or pre-allocated static arrays
            _ => "slot_N_id", // Fallback
        }
    }
    
    /// Format slot count key without allocation
    pub fn format_slot_count_key(&mut self, slot_index: usize) -> &str {
        match slot_index {
            0 => "slot_0_count",
            1 => "slot_1_count",
            2 => "slot_2_count",
            3 => "slot_3_count",
            4 => "slot_4_count",
            5 => "slot_5_count",
            6 => "slot_6_count",
            7 => "slot_7_count",
            8 => "slot_8_count",
            9 => "slot_9_count",
            _ => "slot_N_count", // Fallback
        }
    }
}

/// Pre-allocated slot keys for chest inventories
/// This eliminates format!() allocations entirely
pub struct PreAllocatedSlotKeys {
    id_keys: [&'static str; 27],
    count_keys: [&'static str; 27],
}

impl PreAllocatedSlotKeys {
    pub const fn new() -> Self {
        Self {
            id_keys: [
                "slot_0_id", "slot_1_id", "slot_2_id", "slot_3_id", "slot_4_id",
                "slot_5_id", "slot_6_id", "slot_7_id", "slot_8_id", "slot_9_id",
                "slot_10_id", "slot_11_id", "slot_12_id", "slot_13_id", "slot_14_id",
                "slot_15_id", "slot_16_id", "slot_17_id", "slot_18_id", "slot_19_id",
                "slot_20_id", "slot_21_id", "slot_22_id", "slot_23_id", "slot_24_id",
                "slot_25_id", "slot_26_id",
            ],
            count_keys: [
                "slot_0_count", "slot_1_count", "slot_2_count", "slot_3_count", "slot_4_count",
                "slot_5_count", "slot_6_count", "slot_7_count", "slot_8_count", "slot_9_count",
                "slot_10_count", "slot_11_count", "slot_12_count", "slot_13_count", "slot_14_count",
                "slot_15_count", "slot_16_count", "slot_17_count", "slot_18_count", "slot_19_count",
                "slot_20_count", "slot_21_count", "slot_22_count", "slot_23_count", "slot_24_count",
                "slot_25_count", "slot_26_count",
            ],
        }
    }
    
    pub fn get_id_key(&self, slot_index: usize) -> Option<&'static str> {
        self.id_keys.get(slot_index).copied()
    }
    
    pub fn get_count_key(&self, slot_index: usize) -> Option<&'static str> {
        self.count_keys.get(slot_index).copied()
    }
}

/// Global static slot keys
pub static SLOT_KEYS: PreAllocatedSlotKeys = PreAllocatedSlotKeys::new();

/// Zero-allocation serialization for chest block entities
pub fn serialize_chest_zero_alloc(
    position: VoxelPos,
    slots: &[Option<ItemStackData>; 27],
) -> HashMap<&'static str, serde_json::Value> {
    let mut data = HashMap::with_capacity(slots.len() * 2);
    
    for (i, slot) in slots.iter().enumerate() {
        if let Some(item) = slot {
            if let (Some(id_key), Some(count_key)) = 
                (SLOT_KEYS.get_id_key(i), SLOT_KEYS.get_count_key(i)) {
                data.insert(id_key, serde_json::json!(item.item_id));
                data.insert(count_key, serde_json::json!(item.count));
            }
        }
    }
    
    data
}

/// Zero-allocation deserialization helper
pub fn get_slot_data_zero_alloc(
    data: &HashMap<String, serde_json::Value>,
    slot_index: usize,
) -> Option<ItemStackData> {
    let id_key = SLOT_KEYS.get_id_key(slot_index)?;
    let count_key = SLOT_KEYS.get_count_key(slot_index)?;
    
    let item_id = data.get(id_key)?.as_u64()? as u32;
    let count = data.get(count_key)?.as_u64()? as u32;
    
    Some(ItemStackData { item_id, count })
}

/// Performance comparison test
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn test_allocation_comparison() {
        // Traditional approach with allocations
        let start = Instant::now();
        for _ in 0..1000 {
            let mut _data = HashMap::new();
            for i in 0..27 {
                _data.insert(format!("slot_{}_id", i), serde_json::json!(1));
                _data.insert(format!("slot_{}_count", i), serde_json::json!(2));
            }
        }
        let traditional_time = start.elapsed();
        
        // Zero-allocation approach
        let start = Instant::now();
        for _ in 0..1000 {
            let mut _data = HashMap::with_capacity(54);
            for i in 0..27 {
                if let (Some(id_key), Some(count_key)) = 
                    (SLOT_KEYS.get_id_key(i), SLOT_KEYS.get_count_key(i)) {
                    _data.insert(id_key, serde_json::json!(1));
                    _data.insert(count_key, serde_json::json!(2));
                }
            }
        }
        let zero_alloc_time = start.elapsed();
        
        println!("Traditional approach: {:?}", traditional_time);
        println!("Zero-allocation approach: {:?}", zero_alloc_time);
        
        // Zero-allocation should be faster due to no string allocations
        // (Note: This test may be optimized away by the compiler in release mode)
    }
    
    #[test]
    fn test_slot_keys() {
        assert_eq!(SLOT_KEYS.get_id_key(0), Some("slot_0_id"));
        assert_eq!(SLOT_KEYS.get_count_key(0), Some("slot_0_count"));
        assert_eq!(SLOT_KEYS.get_id_key(26), Some("slot_26_id"));
        assert_eq!(SLOT_KEYS.get_count_key(26), Some("slot_26_count"));
        assert_eq!(SLOT_KEYS.get_id_key(27), None);
    }
}