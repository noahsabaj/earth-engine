// Hybrid decompression shader combining multiple techniques
// Handles blocks with different compression methods

struct CompressionHeader {
    compression_type: u32,
    uncompressed_size: u32,
    compressed_size: u32,
    block_count: u32,
    palette_size: u32,
}

struct BlockHeader {
    compression_method: u32,  // 0=raw, 1=rle, 2=palette
    block_size: u32,
    compressed_size: u32,
    _padding: u32,
}

@group(0) @binding(0) var<storage, read> compressed_data: array<u32>;
@group(0) @binding(1) var<storage, read> header: CompressionHeader;
@group(0) @binding(2) var<storage, read_write> decompressed_data: array<u32>;
@group(0) @binding(3) var<storage, read> block_headers: array<BlockHeader>;

// Workgroup size matches typical block size (8x8x8 = 512 voxels)
@compute @workgroup_size(8, 8, 4)
fn decompress_hybrid(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let block_idx = workgroup_id.x + workgroup_id.y * 16u + workgroup_id.z * 256u;
    
    if (block_idx >= header.block_count) {
        return;
    }
    
    let block_header = block_headers[block_idx];
    let voxel_in_block = local_id.x + local_id.y * 8u + local_id.z * 64u;
    let global_voxel_idx = block_idx * 512u + voxel_in_block;
    
    // Find compressed data offset for this block
    var compressed_offset = 0u;
    for (var i = 0u; i < block_idx; i = i + 1u) {
        compressed_offset = compressed_offset + block_headers[i].compressed_size;
    }
    compressed_offset = compressed_offset / 4u; // Convert to word offset
    
    // Decompress based on method
    switch (block_header.compression_method) {
        case 0u: { // Raw/uncompressed
            if (voxel_in_block < block_header.block_size) {
                decompressed_data[global_voxel_idx] = 
                    compressed_data[compressed_offset + voxel_in_block];
            }
        }
        
        case 1u: { // RLE within block
            decompress_block_rle(
                compressed_offset,
                block_header.compressed_size,
                block_idx,
                voxel_in_block,
                global_voxel_idx
            );
        }
        
        case 2u: { // Palette within block
            decompress_block_palette(
                compressed_offset,
                block_header.compressed_size,
                block_idx,
                voxel_in_block,
                global_voxel_idx
            );
        }
        
        default: {
            // Unknown method - write zero
            decompressed_data[global_voxel_idx] = 0u;
        }
    }
}

// Decompress RLE block
fn decompress_block_rle(
    compressed_offset: u32,
    compressed_size: u32,
    block_idx: u32,
    voxel_in_block: u32,
    global_voxel_idx: u32
) {
    // Simple RLE for small blocks
    var current_voxel = 0u;
    var i = 0u;
    
    while (i < compressed_size / 5u && current_voxel <= voxel_in_block) {
        let run_length = compressed_data[compressed_offset + i * 2u] & 0xFFu;
        let value = compressed_data[compressed_offset + i * 2u + 1u];
        
        if (current_voxel <= voxel_in_block && 
            voxel_in_block < current_voxel + run_length) {
            decompressed_data[global_voxel_idx] = value;
            return;
        }
        
        current_voxel = current_voxel + run_length;
        i = i + 1u;
    }
    
    // If we get here, voxel wasn't found - write zero
    decompressed_data[global_voxel_idx] = 0u;
}

// Decompress palette block
fn decompress_block_palette(
    compressed_offset: u32,
    compressed_size: u32,
    block_idx: u32,
    voxel_in_block: u32,
    global_voxel_idx: u32
) {
    // Read palette size
    let palette_size = compressed_data[compressed_offset] & 0xFFu;
    
    // Read palette entry for this voxel
    if (palette_size <= 16u) {
        // 4-bit indices
        let byte_idx = voxel_in_block / 2u;
        let word_idx = compressed_offset + 1u + palette_size + byte_idx / 4u;
        let byte_in_word = byte_idx % 4u;
        let packed_word = compressed_data[word_idx];
        let packed_byte = (packed_word >> (byte_in_word * 8u)) & 0xFFu;
        
        let index = select(
            packed_byte & 0xFu,
            (packed_byte >> 4u) & 0xFu,
            (voxel_in_block % 2u) == 0u
        );
        
        // Look up in palette
        if (index < palette_size) {
            decompressed_data[global_voxel_idx] = 
                compressed_data[compressed_offset + 1u + index];
        } else {
            decompressed_data[global_voxel_idx] = 0u;
        }
    } else {
        // 8-bit indices
        let byte_idx = voxel_in_block;
        let word_idx = compressed_offset + 1u + palette_size + byte_idx / 4u;
        let byte_in_word = byte_idx % 4u;
        let packed_word = compressed_data[word_idx];
        let index = (packed_word >> (byte_in_word * 8u)) & 0xFFu;
        
        // Look up in palette
        if (index < palette_size) {
            decompressed_data[global_voxel_idx] = 
                compressed_data[compressed_offset + 1u + index];
        } else {
            decompressed_data[global_voxel_idx] = 0u;
        }
    }
}