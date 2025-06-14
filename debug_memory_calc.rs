/// Standalone debug script to calculate memory usage for WorldBuffer
/// This will help us identify the source of the 20GB allocation

const CHUNK_SIZE: u32 = 32;
const VOXELS_PER_CHUNK: u32 = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

fn calculate_buffer_size(world_size: u32) -> (u64, u64, u64) {
    let chunks_total = world_size * world_size * world_size;
    let total_voxels = chunks_total as u64 * VOXELS_PER_CHUNK as u64;
    let voxel_buffer_size = total_voxels * 4; // 4 bytes per voxel
    let metadata_buffer_size = chunks_total as u64 * 16; // 16 bytes per chunk
    (chunks_total as u64, voxel_buffer_size, metadata_buffer_size)
}

fn main() {
    println!("Debugging Earth Engine WorldBuffer memory calculations...");
    println!();
    
    let test_sizes = [8, 16, 32, 64, 128, 256, 512, 1024];
    
    for world_size in test_sizes {
        let (chunks, voxel_size, metadata_size) = calculate_buffer_size(world_size);
        let total_size = voxel_size + metadata_size;
        
        println!("World size: {} chunks per dimension", world_size);
        println!("  Total chunks: {}", chunks);
        println!("  Voxel buffer: {} bytes ({:.2} GB)", voxel_size, voxel_size as f64 / (1024.0 * 1024.0 * 1024.0));
        println!("  Metadata buffer: {} bytes ({:.2} MB)", metadata_size, metadata_size as f64 / (1024.0 * 1024.0));
        println!("  Total: {} bytes ({:.2} GB)", total_size, total_size as f64 / (1024.0 * 1024.0 * 1024.0));
        
        // Check if this matches the 20GB allocation
        if voxel_size == 21474836480 {
            println!("  *** FOUND THE 20GB ALLOCATION! ***");
        }
        
        if total_size > 10_000_000_000 { // > 10GB
            println!("  ⚠️  WARNING: Large allocation detected!");
        }
        println!();
    }
    
    // Reverse calculation to find world_size that gives 20GB
    let target_bytes = 21474836480u64;
    let target_voxels = target_bytes / 4;
    let target_chunks = target_voxels / VOXELS_PER_CHUNK as u64;
    let target_world_size_cubed = target_chunks;
    let target_world_size = (target_world_size_cubed as f64).powf(1.0/3.0);
    
    println!("Reverse calculation for 20GB allocation:");
    println!("  Target bytes: {}", target_bytes);
    println!("  Target voxels: {}", target_voxels);
    println!("  Target chunks: {}", target_chunks);
    println!("  World size (cubed root): {:.2}", target_world_size);
    
    // Check specific values that might cause this
    let check_sizes = [546, 547, 548, 163840];
    for world_size in check_sizes {
        let (chunks, voxel_size, metadata_size) = calculate_buffer_size(world_size);
        println!("  World size {}: {} bytes", world_size, voxel_size);
        if voxel_size == target_bytes {
            println!("    *** MATCH FOUND! ***");
        }
    }
}