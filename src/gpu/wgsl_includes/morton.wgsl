//! Morton encoding functions for GPU shaders
//! 
//! Provides cache-friendly 3D coordinate to 1D index mapping using Z-order curves.
//! This is the single source of truth for GPU morton encoding.

/// Morton encoding for 3D coordinates
/// Interleaves bits of x, y, z coordinates for cache-friendly memory access
/// Supports up to 10 bits per coordinate (max value 1024)
fn morton_encode_3d(x: u32, y: u32, z: u32) -> u32 {
    var xx = x & 0x3FFu;  // Mask to 10 bits
    var yy = y & 0x3FFu;
    var zz = z & 0x3FFu;
    
    // Spread bits using magic bit manipulation
    xx = (xx | (xx << 16u)) & 0x30000FFu;
    xx = (xx | (xx << 8u)) & 0x300F00Fu;
    xx = (xx | (xx << 4u)) & 0x30C30C3u;
    xx = (xx | (xx << 2u)) & 0x9249249u;
    
    yy = (yy | (yy << 16u)) & 0x30000FFu;
    yy = (yy | (yy << 8u)) & 0x300F00Fu;
    yy = (yy | (yy << 4u)) & 0x30C30C3u;
    yy = (yy | (yy << 2u)) & 0x9249249u;
    
    zz = (zz | (zz << 16u)) & 0x30000FFu;
    zz = (zz | (zz << 8u)) & 0x300F00Fu;
    zz = (zz | (zz << 4u)) & 0x30C30C3u;
    zz = (zz | (zz << 2u)) & 0x9249249u;
    
    return xx | (yy << 1u) | (zz << 2u);
}

/// Morton decoding for 3D coordinates
/// Reverses the morton encoding to get back x, y, z coordinates
fn morton_decode_3d(morton: u32) -> vec3<u32> {
    var x = morton & 0x9249249u;
    var y = (morton >> 1u) & 0x9249249u;
    var z = (morton >> 2u) & 0x9249249u;
    
    // Compact bits (reverse of spread operation)
    x = (x ^ (x >> 2u)) & 0x30c30c3u;
    x = (x ^ (x >> 4u)) & 0x300f00fu;
    x = (x ^ (x >> 8u)) & 0x30000ffu;
    x = (x ^ (x >> 16u)) & 0x3ffu;
    
    y = (y ^ (y >> 2u)) & 0x30c30c3u;
    y = (y ^ (y >> 4u)) & 0x300f00fu;
    y = (y ^ (y >> 8u)) & 0x30000ffu;
    y = (y ^ (y >> 16u)) & 0x3ffu;
    
    z = (z ^ (z >> 2u)) & 0x30c30c3u;
    z = (z ^ (z >> 4u)) & 0x300f00fu;
    z = (z ^ (z >> 8u)) & 0x30000ffu;
    z = (z ^ (z >> 16u)) & 0x3ffu;
    
    return vec3<u32>(x, y, z);
}

/// Calculate world buffer index from world position
/// Uses morton encoding for both chunk and voxel indexing
fn world_pos_to_buffer_index(world_pos: vec3<i32>, chunk_size: u32) -> u32 {
    let chunk_pos = world_pos / i32(chunk_size);
    let local_pos = world_pos % i32(chunk_size);
    
    let chunk_morton = morton_encode_3d(
        u32(chunk_pos.x),
        u32(chunk_pos.y),
        u32(chunk_pos.z)
    );
    
    let voxel_morton = morton_encode_3d(
        u32(local_pos.x),
        u32(local_pos.y),
        u32(local_pos.z)
    );
    
    let voxels_per_chunk = chunk_size * chunk_size * chunk_size;
    return chunk_morton * voxels_per_chunk + voxel_morton;
}