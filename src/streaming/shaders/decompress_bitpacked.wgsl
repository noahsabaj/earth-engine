// Bit-packed decompression shader for sparse voxel data
// Decompresses sparse voxel data using bitmap + value list

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

// Workgroup shared memory for bitmap processing
var<workgroup> shared_bitmap: array<u32, 64>; // 2048 bits
var<workgroup> shared_popcounts: array<u32, 64>;
var<workgroup> shared_prefix_sum: array<u32, 64>;

@compute @workgroup_size(64, 1, 1)
fn decompress_bitpacked(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>
) {
    let thread_id = local_id.x;
    let workgroup_idx = workgroup_id.x;
    
    // Read bitmap size from compressed data
    let bitmap_size = compressed_data[0];
    let bitmap_words = (bitmap_size + 3u) / 4u;
    
    // Phase 1: Load bitmap chunk for this workgroup
    let bitmap_offset = workgroup_idx * 64u;
    if (bitmap_offset + thread_id < bitmap_words) {
        shared_bitmap[thread_id] = compressed_data[1u + bitmap_offset + thread_id];
    } else {
        shared_bitmap[thread_id] = 0u;
    }
    
    // Count set bits in each word
    shared_popcounts[thread_id] = countOneBits(shared_bitmap[thread_id]);
    
    workgroupBarrier();
    
    // Phase 2: Prefix sum to find value indices
    if (thread_id == 0u) {
        var sum = 0u;
        for (var i = 0u; i < 64u; i = i + 1u) {
            shared_prefix_sum[i] = sum;
            sum = sum + shared_popcounts[i];
        }
    }
    
    workgroupBarrier();
    
    // Calculate starting value index for this workgroup
    var workgroup_value_offset = 0u;
    if (workgroup_idx > 0u) {
        // Sum all previous workgroups' popcounts
        for (var i = 0u; i < workgroup_idx * 64u; i = i + 1u) {
            if (i < bitmap_words) {
                workgroup_value_offset = workgroup_value_offset + 
                    countOneBits(compressed_data[1u + i]);
            }
        }
    }
    
    // Phase 3: Decompress values for this thread's word
    let my_bitmap = shared_bitmap[thread_id];
    let my_prefix = shared_prefix_sum[thread_id];
    let values_offset = 1u + bitmap_words + 1u; // After bitmap size, bitmap, and value count
    
    // Process each bit in my word
    for (var bit = 0u; bit < 32u; bit = bit + 1u) {
        let voxel_idx = (workgroup_idx * 64u + thread_id) * 32u + bit;
        
        if (voxel_idx < header.uncompressed_size / 4u) {
            if ((my_bitmap & (1u << bit)) != 0u) {
                // Bit is set - read value from compressed data
                let bit_count = countOneBits(my_bitmap & ((1u << bit) - 1u));
                let value_idx = workgroup_value_offset + my_prefix + bit_count;
                decompressed_data[voxel_idx] = compressed_data[values_offset + value_idx];
            } else {
                // Bit is not set - voxel is empty
                decompressed_data[voxel_idx] = 0u;
            }
        }
    }
}