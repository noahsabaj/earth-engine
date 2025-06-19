/// Morton encoding/decoding for 3D coordinates
/// 
/// Uses optimized bit manipulation for fast encoding/decoding

use crate::{VoxelPos, ChunkPos};

/// Magic numbers for bit spreading/compacting
/// These constants are used in the "magic bits" algorithm for fast Morton encoding
const MAGIC_X: u64 = 0x1249249249249249;
const MAGIC_Y: u64 = 0x2492492492492492;
const MAGIC_Z: u64 = 0x4924924924924924;

/// Spreads bits of a 21-bit integer to every 3rd bit
/// Used for Morton encoding
#[inline(always)]
fn spread_bits(v: u32) -> u64 {
    let mut result = 0u64;
    for i in 0..21 {
        if (v >> i) & 1 != 0 {
            result |= 1u64 << (i * 3);
        }
    }
    result
}

/// Compacts every 3rd bit back to a 21-bit integer
/// Used for Morton decoding
#[inline(always)]
fn compact_bits(v: u64) -> u32 {
    let mut result = 0u32;
    for i in 0..21 {
        if (v >> (i * 3)) & 1 != 0 {
            result |= 1u32 << i;
        }
    }
    result
}

/// Encode 3D coordinates into Morton code (Z-order)
/// Supports up to 21 bits per coordinate (2^21 = 2,097,152)
#[inline(always)]
pub fn morton_encode(x: u32, y: u32, z: u32) -> u64 {
    debug_assert!(x < (1 << 21), "x coordinate too large for Morton encoding");
    debug_assert!(y < (1 << 21), "y coordinate too large for Morton encoding");
    debug_assert!(z < (1 << 21), "z coordinate too large for Morton encoding");
    
    spread_bits(x) | (spread_bits(y) << 1) | (spread_bits(z) << 2)
}

/// Decode Morton code back to 3D coordinates
#[inline(always)]
pub fn morton_decode(morton: u64) -> (u32, u32, u32) {
    let x = compact_bits(morton);
    let y = compact_bits(morton >> 1);
    let z = compact_bits(morton >> 2);
    (x, y, z)
}

/// Encode chunk-relative voxel position
/// Optimized for 50x50x50 chunks (1dcm³ voxels)
#[inline(always)]
pub fn morton_encode_chunk(pos: VoxelPos) -> u32 {
    // For chunk-relative positions, we need 6 bits per coordinate (2^6 = 64 > 50)
    debug_assert!(pos.x < 50 && pos.y < 50 && pos.z < 50);
    
    let x = pos.x as u32;
    let y = pos.y as u32;
    let z = pos.z as u32;
    
    // Optimized bit interleaving for 6-bit values
    let mut result = 0u32;
    for i in 0..6 {
        result |= ((x >> i) & 1) << (i * 3);
        result |= ((y >> i) & 1) << (i * 3 + 1);
        result |= ((z >> i) & 1) << (i * 3 + 2);
    }
    result
}

/// Decode chunk-relative Morton code
#[inline(always)]
pub fn morton_decode_chunk(morton: u32) -> VoxelPos {
    let mut x = 0u32;
    let mut y = 0u32;
    let mut z = 0u32;
    
    // Extract interleaved bits
    for i in 0..6 {
        x |= ((morton >> (i * 3)) & 1) << i;
        y |= ((morton >> (i * 3 + 1)) & 1) << i;
        z |= ((morton >> (i * 3 + 2)) & 1) << i;
    }
    
    VoxelPos {
        x: x as i32,
        y: y as i32,
        z: z as i32,
    }
}

/// Iterator for Morton-ordered traversal of a 3D region
/// This ensures cache-friendly access patterns
pub struct MortonIterator {
    start: u64,
    end: u64,
    current: u64,
}

impl MortonIterator {
    pub fn new(min: (u32, u32, u32), max: (u32, u32, u32)) -> Self {
        Self {
            start: morton_encode(min.0, min.1, min.2),
            end: morton_encode(max.0, max.1, max.2),
            current: morton_encode(min.0, min.1, min.2),
        }
    }
}

impl Iterator for MortonIterator {
    type Item = (u32, u32, u32);
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.current > self.end {
            return None;
        }
        
        let result = morton_decode(self.current);
        self.current += 1;
        
        // Skip values outside our bounds (Morton curve visits extra points)
        while self.current <= self.end {
            let (x, y, z) = morton_decode(self.current);
            let (min_x, min_y, min_z) = morton_decode(self.start);
            let (max_x, max_y, max_z) = morton_decode(self.end);
            
            if x >= min_x && x <= max_x && 
               y >= min_y && y <= max_y && 
               z >= min_z && z <= max_z {
                break;
            }
            self.current += 1;
        }
        
        Some(result)
    }
}

/// Convert world position to Morton code
pub fn world_pos_to_morton(chunk: ChunkPos, voxel: VoxelPos) -> u64 {
    let world_x = (chunk.x * 50 + voxel.x) as u32;
    let world_y = (chunk.y * 50 + voxel.y) as u32;
    let world_z = (chunk.z * 50 + voxel.z) as u32;
    morton_encode(world_x, world_y, world_z)
}

/// Convert Morton code to world position
pub fn morton_to_world_pos(morton: u64) -> (ChunkPos, VoxelPos) {
    let (x, y, z) = morton_decode(morton);
    let chunk = ChunkPos {
        x: (x / 50) as i32,
        y: (y / 50) as i32,
        z: (z / 50) as i32,
    };
    let voxel = VoxelPos {
        x: (x % 50) as i32,
        y: (y % 50) as i32,
        z: (z % 50) as i32,
    };
    (chunk, voxel)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_morton_encode_decode() {
        let test_cases = [
            (0, 0, 0),
            (1, 1, 1),
            (7, 7, 7),
            (15, 15, 15),
            (100, 200, 50),
            (1000, 2000, 500),
        ];
        
        for (x, y, z) in test_cases {
            let morton = morton_encode(x, y, z);
            let (dx, dy, dz) = morton_decode(morton);
            assert_eq!((x, y, z), (dx, dy, dz), 
                "Failed for ({}, {}, {})", x, y, z);
        }
    }
    
    #[test]
    fn test_chunk_morton() {
        for x in 0..50 {
            for y in 0..50 {
                for z in 0..50 {
                    let pos = VoxelPos { x, y, z };
                    let morton = morton_encode_chunk(pos);
                    let decoded = morton_decode_chunk(morton);
                    assert_eq!(pos, decoded);
                }
            }
        }
    }
    
    #[test]
    fn test_morton_locality() {
        // Test that nearby coordinates have nearby Morton codes
        let base = morton_encode(100, 100, 100);
        let neighbor = morton_encode(101, 100, 100);
        
        // Morton codes should be relatively close
        assert!((neighbor as i64 - base as i64).abs() < 100);
    }
}