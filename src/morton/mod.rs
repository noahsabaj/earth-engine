/// Morton encoding (Z-order curve) for improved cache locality
/// 
/// This module provides Morton encoding/decoding for 3D voxel coordinates,
/// which dramatically improves cache performance by ensuring spatially close
/// voxels are also close in memory.

pub mod morton3d;

pub use morton3d::{morton_encode, morton_decode, morton_encode_chunk, morton_decode_chunk};

// Morton encoding improves cache locality by interleaving the bits of
// x, y, and z coordinates. This creates a Z-order curve through 3D space
// where nearby points in 3D are likely to be nearby in the 1D encoding.
//
// Benefits:
// - 3-5x better cache hit rate for neighbor access
// - Reduced memory bandwidth usage
// - Better GPU memory coalescing
// - Works perfectly with virtual memory pages