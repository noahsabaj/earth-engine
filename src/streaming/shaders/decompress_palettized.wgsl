// Palettized decompression shader for voxel data
// Decompresses palette-indexed voxel data

struct CompressionHeader {
    compression_type: u32,
    uncompressed_size: u32,
    compressed_size: u32,
    block_count: u32,
    palette_size: u32,
}

@group(0) @binding(0) var<storage, read> compressed_data: array<u32>;
@group(0) @binding(1) var<storage, read> header: CompressionHeader;
@group(0) @binding(2) var<storage, read_write> decompressed_data: array<u32>;

// Workgroup shared memory for palette
var<workgroup> shared_palette: array<u32, 256>;

@compute @workgroup_size(256, 1, 1)
fn decompress_palettized(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let thread_id = local_id.x;
    let global_thread = global_id.x;
    
    // Phase 1: Load palette into shared memory
    let palette_size = header.palette_size;
    if (thread_id < palette_size) {
        shared_palette[thread_id] = compressed_data[1u + thread_id];
    }
    
    workgroupBarrier();
    
    // Calculate indices offset (after palette size and palette data)
    let indices_offset = 1u + palette_size;
    
    // Phase 2: Decompress indices
    if (palette_size <= 16u) {
        // 4-bit indices - each byte contains 2 indices
        let byte_idx = global_thread / 2u;
        let is_high_nibble = (global_thread % 2u) == 0u;
        
        if (byte_idx < (header.uncompressed_size / 4u + 1u) / 2u) {
            // Read packed byte
            let word_idx = indices_offset + byte_idx / 4u;
            let byte_in_word = byte_idx % 4u;
            let packed_word = compressed_data[word_idx];
            let packed_byte = (packed_word >> (byte_in_word * 8u)) & 0xFFu;
            
            // Extract index
            let index = select(
                packed_byte & 0xFu,          // Low nibble
                (packed_byte >> 4u) & 0xFu,  // High nibble
                is_high_nibble
            );
            
            // Write decompressed value
            if (global_thread < header.uncompressed_size / 4u) {
                decompressed_data[global_thread] = shared_palette[index];
            }
        }
    } else {
        // 8-bit indices
        let byte_idx = global_thread;
        
        if (byte_idx < header.uncompressed_size / 4u) {
            // Read index byte
            let word_idx = indices_offset + byte_idx / 4u;
            let byte_in_word = byte_idx % 4u;
            let packed_word = compressed_data[word_idx];
            let index = (packed_word >> (byte_in_word * 8u)) & 0xFFu;
            
            // Write decompressed value
            decompressed_data[global_thread] = shared_palette[index];
        }
    }
}