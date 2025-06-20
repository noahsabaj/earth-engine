use crate::world::core::{BlockId, ChunkPos, VoxelPos};
use crate::world::lighting::LightLevel;
use crate::world::interfaces::ChunkData;
use crate::morton::morton_encode_chunk;
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ptr;

/// Cache line size for most modern CPUs
const CACHE_LINE_SIZE: usize = 64;

/// Chunk data in Structure-of-Arrays format with cache alignment
/// 
/// This layout provides:
/// - Better cache efficiency when accessing single attributes
/// - SIMD-friendly data layout
/// - Reduced memory bandwidth for partial updates
/// - Cache-aligned arrays for optimal performance
#[derive(Clone, Debug)]
pub struct ChunkSoA {
    position: ChunkPos,
    size: u32,
    voxel_count: usize,
    
    // Morton-ordered, cache-aligned arrays
    block_ids: AlignedArray<BlockId>,
    sky_light: AlignedArray<u8>,
    block_light: AlignedArray<u8>,
    material_flags: AlignedArray<u8>,
    
    // Metadata
    dirty: bool,
    light_dirty: bool,
}

/// Cache-aligned array wrapper
struct AlignedArray<T> {
    ptr: *mut T,
    len: usize,
    layout: Layout,
}

impl<T: Copy + Default> AlignedArray<T> {
    /// Create a new cache-aligned array
    fn new(len: usize) -> Self {
        let size = len * std::mem::size_of::<T>();
        let align = CACHE_LINE_SIZE.max(std::mem::align_of::<T>());
        
        let layout = Layout::from_size_align(size, align)
            .expect("Invalid layout");
        
        // SAFETY: We're allocating aligned memory with proper layout
        // - Layout is valid (checked by from_size_align)
        // - We check for null pointer after allocation
        // - We initialize all memory before use
        // - The allocated memory is owned by this struct and freed in Drop
        unsafe {
            let ptr = alloc_zeroed(layout) as *mut T;
            if ptr.is_null() {
                panic!("Failed to allocate aligned memory");
            }
            
            // Initialize with default values
            for i in 0..len {
                ptr::write(ptr.add(i), T::default());
            }
            
            Self { ptr, len, layout }
        }
    }
    
    /// Get element at Morton-encoded index
    #[inline(always)]
    unsafe fn get_unchecked(&self, index: usize) -> T {
        // SAFETY: Caller must ensure index < self.len
        // - The memory at ptr + index is valid and initialized
        // - T is Copy, so we can safely read the value
        *self.ptr.add(index)
    }
    
    /// Set element at Morton-encoded index
    #[inline(always)]
    unsafe fn set_unchecked(&mut self, index: usize, value: T) {
        // SAFETY: Caller must ensure index < self.len
        // - The memory at ptr + index is valid and initialized
        // - We have exclusive mutable access to self
        // - T is Copy, so we can safely write the value
        *self.ptr.add(index) = value;
    }
    
    /// Get mutable slice for bulk operations
    unsafe fn as_mut_slice(&mut self) -> &mut [T] {
        // SAFETY: We own the memory pointed to by ptr
        // - ptr is non-null and properly aligned
        // - Memory from ptr to ptr + len is allocated and initialized
        // - We have exclusive mutable access
        // - The lifetime is tied to &mut self
        std::slice::from_raw_parts_mut(self.ptr, self.len)
    }
}

impl<T: Copy + Default> Clone for AlignedArray<T> {
    fn clone(&self) -> Self {
        let new_array = Self::new(self.len);
        
        // SAFETY: Both arrays have the same length and valid memory
        // - Both pointers are valid for self.len elements
        // - T is Copy, so we can safely copy the data
        unsafe {
            std::ptr::copy_nonoverlapping(self.ptr, new_array.ptr, self.len);
        }
        
        new_array
    }
}

impl<T> Drop for AlignedArray<T> {
    fn drop(&mut self) {
        // SAFETY: We're deallocating memory we allocated
        // - ptr was allocated with alloc_zeroed using self.layout
        // - layout hasn't changed since allocation
        // - This is the only place we deallocate this memory
        unsafe {
            dealloc(self.ptr as *mut u8, self.layout);
        }
    }
}

// Safety: AlignedArray owns its data and doesn't allow shared mutable access
unsafe impl<T: Send> Send for AlignedArray<T> {}

impl<T> std::fmt::Debug for AlignedArray<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlignedArray")
            .field("len", &self.len)
            .field("ptr", &self.ptr)
            .finish()
    }
}
unsafe impl<T: Sync> Sync for AlignedArray<T> {}

impl ChunkSoA {
    pub fn new(position: ChunkPos, size: u32) -> Self {
        let voxel_count = (size * size * size) as usize;
        
        Self {
            position,
            size,
            voxel_count,
            block_ids: AlignedArray::new(voxel_count),
            sky_light: AlignedArray::new(voxel_count),
            block_light: AlignedArray::new(voxel_count),
            material_flags: AlignedArray::new(voxel_count),
            dirty: true,
            light_dirty: true,
        }
    }
    
    /// Get block at position using Morton encoding
    #[inline(always)]
    pub fn get_block(&self, x: u32, y: u32, z: u32) -> BlockId {
        if x >= self.size || y >= self.size || z >= self.size {
            return BlockId::AIR;
        }
        
        let morton_idx = morton_encode_chunk(VoxelPos {
            x: x as i32,
            y: y as i32,
            z: z as i32,
        }) as usize;
        
        // SAFETY: morton_idx is guaranteed to be < voxel_count
        // - We've already checked x, y, z are within bounds
        // - Morton encoding preserves the valid index range
        unsafe { self.block_ids.get_unchecked(morton_idx) }
    }
    
    /// Set block at position using Morton encoding
    #[inline(always)]
    pub fn set_block(&mut self, x: u32, y: u32, z: u32, block: BlockId) {
        if x >= self.size || y >= self.size || z >= self.size {
            return;
        }
        
        let morton_idx = morton_encode_chunk(VoxelPos {
            x: x as i32,
            y: y as i32,
            z: z as i32,
        }) as usize;
        
        // SAFETY: morton_idx is guaranteed to be < voxel_count
        // - We've already checked x, y, z are within bounds
        // - Morton encoding preserves the valid index range
        // - We have exclusive mutable access to self
        unsafe {
            self.block_ids.set_unchecked(morton_idx, block);
        }
        self.dirty = true;
    }
    
    /// Get light levels
    #[inline(always)]
    pub fn get_light(&self, x: u32, y: u32, z: u32) -> LightLevel {
        if x >= self.size || y >= self.size || z >= self.size {
            return LightLevel { sky: 15, block: 0 };
        }
        
        let morton_idx = morton_encode_chunk(VoxelPos {
            x: x as i32,
            y: y as i32,
            z: z as i32,
        }) as usize;
        
        // SAFETY: morton_idx is guaranteed to be < voxel_count
        // - We've already checked x, y, z are within bounds
        // - Morton encoding preserves the valid index range
        unsafe {
            LightLevel {
                sky: self.sky_light.get_unchecked(morton_idx),
                block: self.block_light.get_unchecked(morton_idx),
            }
        }
    }
    
    /// Set block ID directly by index (for generation)
    #[inline(always)]
    pub fn set_block_by_index(&mut self, index: usize, block_id: BlockId) {
        if index < self.voxel_count {
            // SAFETY: We've checked the index is valid
            unsafe {
                self.block_ids.set_unchecked(index, block_id);
            }
        }
    }
    
    /// Set light levels
    #[inline(always)]
    pub fn set_light(&mut self, x: u32, y: u32, z: u32, light: LightLevel) {
        if x >= self.size || y >= self.size || z >= self.size {
            return;
        }
        
        let morton_idx = morton_encode_chunk(VoxelPos {
            x: x as i32,
            y: y as i32,
            z: z as i32,
        }) as usize;
        
        // SAFETY: morton_idx is guaranteed to be < voxel_count
        // - We've already checked x, y, z are within bounds
        // - Morton encoding preserves the valid index range
        // - We have exclusive mutable access to self
        unsafe {
            self.sky_light.set_unchecked(morton_idx, light.sky);
            self.block_light.set_unchecked(morton_idx, light.block);
        }
        self.light_dirty = true;
        self.dirty = true;
    }
    
    /// Bulk update operations for better cache usage
    pub fn update_region<F>(&mut self, min: VoxelPos, max: VoxelPos, mut updater: F)
    where
        F: FnMut(u32, u32, u32, BlockId) -> BlockId,
    {
        // Process in Morton order for cache efficiency
        for z in min.z..=max.z {
            for y in min.y..=max.y {
                for x in min.x..=max.x {
                    if x >= 0 && y >= 0 && z >= 0 &&
                       x < self.size as i32 && y < self.size as i32 && z < self.size as i32 {
                        let old_block = self.get_block(x as u32, y as u32, z as u32);
                        let new_block = updater(x as u32, y as u32, z as u32, old_block);
                        if old_block != new_block {
                            self.set_block(x as u32, y as u32, z as u32, new_block);
                        }
                    }
                }
            }
        }
    }
    
    /// Process blocks in cache-friendly order
    pub fn iter_blocks_morton<F>(&self, mut processor: F)
    where
        F: FnMut(u32, u32, u32, BlockId),
    {
        // SAFETY: We iterate within bounds 0..voxel_count
        // - All morton_idx values are < voxel_count
        // - block_ids array has voxel_count elements
        unsafe {
            for morton_idx in 0..self.voxel_count {
                let block = self.block_ids.get_unchecked(morton_idx);
                
                // Decode Morton index
                let pos = crate::morton::morton_decode_chunk(morton_idx as u32);
                processor(pos.x as u32, pos.y as u32, pos.z as u32, block);
            }
        }
    }
    
    /// Prefetch data for upcoming access
    #[inline(always)]
    pub fn prefetch_region(&self, center_x: u32, center_y: u32, center_z: u32, radius: u32) {
        // Use volatile reads to hint at upcoming access patterns
        // This is a portable alternative to prefetch intrinsics
        
        for dz in -(radius as i32)..=(radius as i32) {
            for dy in -(radius as i32)..=(radius as i32) {
                for dx in -(radius as i32)..=(radius as i32) {
                    let x = center_x as i32 + dx;
                    let y = center_y as i32 + dy;
                    let z = center_z as i32 + dz;
                    
                    if x >= 0 && y >= 0 && z >= 0 &&
                       x < self.size as i32 && y < self.size as i32 && z < self.size as i32 {
                        let morton_idx = morton_encode_chunk(VoxelPos { x, y, z }) as usize;
                        
                        // SAFETY: morton_idx is within bounds
                        // - We've checked x, y, z are within chunk bounds
                        // - Morton encoding preserves valid indices
                        // - read_volatile is safe for initialized memory
                        // - We're only reading, not modifying
                        unsafe {
                            // Touch the memory to bring it into cache
                            let _ = std::ptr::read_volatile(self.block_ids.ptr.add(morton_idx));
                            let _ = std::ptr::read_volatile(self.sky_light.ptr.add(morton_idx));
                        }
                    }
                }
            }
        }
    }
    
    // Getters
    pub fn position(&self) -> ChunkPos { self.position }
    pub fn size(&self) -> u32 { self.size }
    pub fn is_dirty(&self) -> bool { self.dirty }
    pub fn is_light_dirty(&self) -> bool { self.light_dirty }
    pub fn mark_clean(&mut self) { self.dirty = false; }
    pub fn mark_light_clean(&mut self) { self.light_dirty = false; }
    pub fn mark_dirty(&mut self) { self.dirty = true; }
    pub fn clear_dirty(&mut self) { self.dirty = false; }
    
    /// Get block using VoxelPos (assumes local coordinates)
    pub fn get_block_at(&self, pos: VoxelPos) -> BlockId {
        self.get_block(pos.x as u32, pos.y as u32, pos.z as u32)
    }
    
    /// Set block using VoxelPos (assumes local coordinates)
    pub fn set_block_at(&mut self, pos: VoxelPos, block: BlockId) {
        self.set_block(pos.x as u32, pos.y as u32, pos.z as u32, block);
    }
    
    /// Get sky light level
    pub fn get_sky_light(&self, x: u32, y: u32, z: u32) -> u8 {
        self.get_light(x, y, z).sky
    }
    
    /// Set sky light level
    pub fn set_sky_light(&mut self, x: u32, y: u32, z: u32, level: u8) {
        let mut light = self.get_light(x, y, z);
        light.sky = level;
        self.set_light(x, y, z, light);
    }
    
    /// Get block light level
    pub fn get_block_light(&self, x: u32, y: u32, z: u32) -> u8 {
        self.get_light(x, y, z).block
    }
    
    /// Set block light level
    pub fn set_block_light(&mut self, x: u32, y: u32, z: u32, level: u8) {
        let mut light = self.get_light(x, y, z);
        light.block = level;
        self.set_light(x, y, z, light);
    }
    
    /// Get all blocks for iteration (returns a Vec for compatibility)
    pub fn blocks(&self) -> Vec<BlockId> {
        let mut blocks = Vec::with_capacity(self.voxel_count);
        
        // Extract blocks in linear order (not Morton order) for compatibility
        for z in 0..self.size {
            for y in 0..self.size {
                for x in 0..self.size {
                    blocks.push(self.get_block(x, y, z));
                }
            }
        }
        
        blocks
    }
}

/// Memory usage statistics
impl ChunkSoA {
    pub fn memory_stats(&self) -> ChunkMemoryStats {
        ChunkMemoryStats {
            voxel_count: self.voxel_count,
            block_ids_size: self.voxel_count * std::mem::size_of::<BlockId>(),
            sky_light_size: self.voxel_count * std::mem::size_of::<u8>(),
            block_light_size: self.voxel_count * std::mem::size_of::<u8>(),
            material_flags_size: self.voxel_count * std::mem::size_of::<u8>(),
            alignment_overhead: CACHE_LINE_SIZE * 4, // Approximate
        }
    }
}

#[derive(Debug)]
pub struct ChunkMemoryStats {
    pub voxel_count: usize,
    pub block_ids_size: usize,
    pub sky_light_size: usize,
    pub block_light_size: usize,
    pub material_flags_size: usize,
    pub alignment_overhead: usize,
}

impl ChunkMemoryStats {
    pub fn total_size(&self) -> usize {
        self.block_ids_size + 
        self.sky_light_size + 
        self.block_light_size + 
        self.material_flags_size + 
        self.alignment_overhead
    }
}

// Implement ChunkData trait for ChunkSoA
impl ChunkData for ChunkSoA {
    fn position(&self) -> ChunkPos {
        self.position
    }
    
    fn get_block_at(&self, x: u32, y: u32, z: u32) -> BlockId {
        self.get_block(x, y, z)
    }
    
    fn set_block_at(&mut self, x: u32, y: u32, z: u32, block: BlockId) {
        self.set_block(x, y, z, block);
    }
    
    fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    fn mark_clean(&mut self) {
        self.dirty = false;
        self.light_dirty = false;
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chunk_soa() {
        let mut chunk = ChunkSoA::new(ChunkPos::new(0, 0, 0), 50);
        
        // Test basic operations
        chunk.set_block(10, 20, 15, BlockId::STONE);
        assert_eq!(chunk.get_block(10, 20, 15), BlockId::STONE);
        
        // Test light
        chunk.set_light(10, 20, 15, LightLevel { sky: 10, block: 5 });
        let light = chunk.get_light(10, 20, 15);
        assert_eq!(light.sky, 10);
        assert_eq!(light.block, 5);
    }
    
    #[test]
    fn test_cache_alignment() {
        let chunk = ChunkSoA::new(ChunkPos::new(0, 0, 0), 50);
        
        // Verify alignment
        assert_eq!(chunk.block_ids.ptr as usize % CACHE_LINE_SIZE, 0);
        assert_eq!(chunk.sky_light.ptr as usize % CACHE_LINE_SIZE, 0);
        assert_eq!(chunk.block_light.ptr as usize % CACHE_LINE_SIZE, 0);
    }
}