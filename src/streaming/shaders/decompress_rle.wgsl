// RLE decompression shader for voxel data
// Decompresses run-length encoded voxel data directly on GPU

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
@group(0) @binding(3) var<storage, read_write> decode_state: array<atomic<u32>>;

// Workgroup shared memory for cooperative decompression
var<workgroup> shared_runs: array<u32, 256>;
var<workgroup> shared_values: array<u32, 256>;
var<workgroup> shared_offsets: array<u32, 256>;

@compute @workgroup_size(256, 1, 1)
fn decompress_rle(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let thread_id = local_id.x;
    let workgroup_idx = workgroup_id.x;
    
    // Phase 1: Parallel scan to find run boundaries
    let compressed_idx = workgroup_idx * 256u + thread_id;
    
    if (compressed_idx < header.compressed_size / 5u) { // 5 bytes per RLE entry
        // Read RLE entry (1 byte count + 4 byte value)
        let byte_offset = compressed_idx * 5u;
        let word_offset = byte_offset / 4u;
        let byte_in_word = byte_offset % 4u;
        
        // Extract count (1 byte)
        let count_word = compressed_data[word_offset];
        let count = (count_word >> (byte_in_word * 8u)) & 0xFFu;
        
        // Extract value (4 bytes)
        let value_offset = byte_offset + 1u;
        let value = read_unaligned_u32(value_offset);
        
        shared_runs[thread_id] = count;
        shared_values[thread_id] = value;
    } else {
        shared_runs[thread_id] = 0u;
        shared_values[thread_id] = 0u;
    }
    
    workgroupBarrier();
    
    // Phase 2: Prefix sum to calculate output offsets
    var offset = 0u;
    if (thread_id == 0u) {
        for (var i = 0u; i < 256u; i = i + 1u) {
            shared_offsets[i] = offset;
            offset = offset + shared_runs[i];
        }
    }
    
    workgroupBarrier();
    
    // Phase 3: Parallel decompression
    let run_count = shared_runs[thread_id];
    let run_value = shared_values[thread_id];
    let output_offset = shared_offsets[thread_id];
    
    // Each thread writes its run
    for (var i = 0u; i < run_count; i = i + 1u) {
        let output_idx = output_offset + i;
        if (output_idx < header.uncompressed_size / 4u) {
            decompressed_data[output_idx] = run_value;
        }
    }
}

// Helper function to read unaligned u32
fn read_unaligned_u32(byte_offset: u32) -> u32 {
    let word_offset = byte_offset / 4u;
    let byte_in_word = byte_offset % 4u;
    
    if (byte_in_word == 0u) {
        return compressed_data[word_offset + 1u];
    } else {
        let word0 = compressed_data[word_offset];
        let word1 = compressed_data[word_offset + 1u];
        
        let shift = byte_in_word * 8u;
        return (word0 >> shift) | (word1 << (32u - shift));
    }
}