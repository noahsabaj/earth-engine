use crate::item::ItemId;
use serde::{Serialize, Deserialize};

/// Maximum items in a single stack
pub const MAX_STACK_SIZE: u32 = 64;

/// Represents a stack of items
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item_id: ItemId,
    pub count: u32,
}

impl ItemStack {
    /// Create a new item stack
    pub fn new(item_id: ItemId, count: u32) -> Self {
        Self {
            item_id,
            count: count.min(MAX_STACK_SIZE),
        }
    }
    
    /// Create a single item
    pub fn single(item_id: ItemId) -> Self {
        Self::new(item_id, 1)
    }
    
    /// Check if this stack can merge with another
    pub fn can_merge_with(&self, other: &ItemStack) -> bool {
        self.item_id == other.item_id && self.count < MAX_STACK_SIZE
    }
    
    /// Try to add items to this stack, returns remaining items
    pub fn try_add(&mut self, count: u32) -> u32 {
        let space = MAX_STACK_SIZE - self.count;
        let to_add = count.min(space);
        self.count += to_add;
        count - to_add
    }
    
    /// Try to merge with another stack, returns true if any items were merged
    pub fn try_merge(&mut self, other: &mut ItemStack) -> bool {
        if !self.can_merge_with(other) {
            return false;
        }
        
        let remaining = self.try_add(other.count);
        other.count = remaining;
        remaining < other.count
    }
    
    /// Split the stack, taking up to the specified count
    pub fn split(&mut self, count: u32) -> Option<ItemStack> {
        if count >= self.count {
            // Take the whole stack
            let result = self.clone();
            self.count = 0;
            Some(result)
        } else if count > 0 {
            // Split the stack
            self.count -= count;
            Some(ItemStack::new(self.item_id, count))
        } else {
            None
        }
    }
    
    /// Check if stack is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    
    /// Check if stack is full
    pub fn is_full(&self) -> bool {
        self.count >= MAX_STACK_SIZE
    }
    
    /// Get the maximum stack size for this item type
    pub fn max_stack_size(&self) -> u32 {
        // In the future, different items might have different max stack sizes
        // For now, all items stack to 64
        MAX_STACK_SIZE
    }
}