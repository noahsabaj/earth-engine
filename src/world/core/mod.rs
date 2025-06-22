//! Core world data types and fundamental structures
//!
//! This module contains the essential data types that form the foundation
//! of the world system, independent of whether CPU or GPU backend is used.

mod basic_blocks;
mod block;
mod position;
mod ray;
mod registry;

pub use block::{Block, BlockId, PhysicsProperties, RenderData};
pub use position::{ChunkPos, VoxelPos};
pub use ray::{cast_ray, BlockFace, Ray, RaycastHit};
pub use registry::BlockRegistry;

// Re-export basic block definitions
pub use basic_blocks::{register_basic_blocks, AirBlock, GrassBlock, StoneBlock};
