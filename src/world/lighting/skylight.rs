//! Skylight calculation for the unified world system
//!
//! This module provides skylight propagation and column updates
//! compatible with the GPU-first architecture.

use crate::world::core::{BlockId, VoxelPos};
use crate::world::{functional_wrapper, interfaces::WorldInterface};

/// Skylight calculator - provides column-based skylight updates
pub struct SkylightCalculator;

impl SkylightCalculator {
    /// Update skylight values for a vertical column
    ///
    /// This recalculates skylight propagation when blocks change.
    /// In the GPU-first architecture, this would typically be done on GPU.
    pub fn update_column<W: WorldInterface>(world: &mut W, x: i32, _y: i32, z: i32) {
        // Start from the top of the world
        let mut current_light = 15u8; // Full skylight at top

        // Scan down the column
        for y in (0..256).rev() {
            let pos = VoxelPos::new(x, y, z);
            let block = functional_wrapper::get_block(world, pos);

            // Air blocks get full skylight from above
            if block == BlockId::AIR {
                // In a full implementation, we'd set skylight value here
                // For now, just track it
                current_light = 15;
            } else if is_transparent(world, block) {
                // Transparent blocks reduce light slightly
                current_light = current_light.saturating_sub(1);
            } else {
                // Opaque blocks block all skylight
                current_light = 0;
            }

            // In a full implementation, we'd propagate horizontally here
        }
    }

    /// Update skylight for a specific position and its neighbors
    pub fn update_at_position<W: WorldInterface>(world: &mut W, pos: VoxelPos) {
        // Update the column containing this position
        Self::update_column(world, pos.x, pos.y, pos.z);

        // Also update neighboring columns that might be affected
        for dx in -1..=1 {
            for dz in -1..=1 {
                if dx != 0 || dz != 0 {
                    Self::update_column(world, pos.x + dx, pos.y, pos.z + dz);
                }
            }
        }
    }
}

/// Helper function to check if a block is transparent for skylight
fn is_transparent<W: WorldInterface>(world: &W, block_id: BlockId) -> bool {
    // Water is transparent but dims light
    if block_id == BlockId::WATER {
        return true;
    }

    // Glass and similar blocks would be transparent
    if block_id == BlockId::GLASS {
        return true;
    }

    // Most blocks are opaque
    false
}
