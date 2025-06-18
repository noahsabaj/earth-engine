//! Core world data types and fundamental structures
//!
//! This module contains the essential data types that form the foundation
//! of the world system, independent of whether CPU or GPU backend is used.

mod block;
mod position;
mod ray;
mod registry;

pub use block::{Block, BlockId, RenderData, PhysicsProperties};
pub use position::{ChunkPos, VoxelPos};
pub use ray::{Ray, RaycastHit, BlockFace, cast_ray};
pub use registry::BlockRegistry;

// Re-export basic block definitions
pub use block::{AirBlock, StoneBlock, GrassBlock, register_basic_blocks};