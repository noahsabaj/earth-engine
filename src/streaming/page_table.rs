use bytemuck::{Pod, Zeroable};

/// Flags for page table entries
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageFlags {
    Empty = 0,
    Resident = 1 << 0,      // Page is loaded in GPU memory
    Dirty = 1 << 1,         // Page has been modified
    Locked = 1 << 2,        // Page cannot be evicted
    Compressed = 1 << 3,    // Page is compressed on disk
    Streaming = 1 << 4,     // Page is currently being loaded
}

/// Page table entry - pure data structure
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PageTableEntry {
    /// Offset in GPU WorldBuffer (u64::MAX if not resident)
    pub physical_offset: u64,
    
    /// Offset in world file on disk
    pub disk_offset: u64,
    
    /// Compression type (0=none, 1=RLE, 2=custom GPU)
    pub compression_type: u8,
    
    /// Access count for LRU tracking
    pub access_count: u16,
    
    /// Page flags (resident, dirty, locked, etc)
    pub flags: u8,
    
    /// Compressed size in bytes (0 if uncompressed)
    pub compressed_size: u32,
}

impl PageTableEntry {
    pub const INVALID_OFFSET: u64 = u64::MAX;
    
    /// Create an empty page entry
    pub fn empty() -> Self {
        Self {
            physical_offset: Self::INVALID_OFFSET,
            disk_offset: 0,
            compression_type: 0,
            access_count: 0,
            flags: PageFlags::Empty as u8,
            compressed_size: 0,
        }
    }
    
    /// Check if page is resident in memory
    pub fn is_resident(&self) -> bool {
        self.flags & PageFlags::Resident as u8 != 0
    }
    
    /// Check if page is dirty
    pub fn is_dirty(&self) -> bool {
        self.flags & PageFlags::Dirty as u8 != 0
    }
    
    /// Check if page is locked
    pub fn is_locked(&self) -> bool {
        self.flags & PageFlags::Locked as u8 != 0
    }
}

/// Hierarchical page table for sparse worlds
#[repr(C)]
#[derive(Debug)]
pub struct PageTable {
    /// Flat array of page entries
    pub entries: Vec<PageTableEntry>,
    
    /// Page size in voxels (typically 64)
    pub page_size: u32,
    
    /// World bounds in pages
    pub world_size_pages: (u32, u32, u32),
    
    /// Total pages allocated
    pub total_pages: u64,
    
    /// Currently resident pages
    pub resident_pages: u32,
    
    /// Hierarchical index for sparse worlds (optional)
    pub sparse_index: Option<SparseIndex>,
}

/// Sparse index for efficient empty space skipping
#[derive(Debug)]
pub struct SparseIndex {
    /// Level 0: 8x8x8 page groups
    pub level0: Vec<u64>,
    
    /// Level 1: 64x64x64 page groups  
    pub level1: Vec<u32>,
    
    /// Level 2: 512x512x512 page groups
    pub level2: Vec<u16>,
}

impl PageTable {
    /// Calculate page index from world position
    pub fn page_index(&self, page_x: u32, page_y: u32, page_z: u32) -> Option<usize> {
        if page_x >= self.world_size_pages.0 ||
           page_y >= self.world_size_pages.1 ||
           page_z >= self.world_size_pages.2 {
            return None;
        }
        
        let index = page_x as usize +
                   (page_y as usize * self.world_size_pages.0 as usize) +
                   (page_z as usize * self.world_size_pages.0 as usize * self.world_size_pages.1 as usize);
        
        Some(index)
    }
    
    /// Get page coordinates from voxel position
    pub fn voxel_to_page(&self, voxel_x: u32, voxel_y: u32, voxel_z: u32) -> (u32, u32, u32) {
        (
            voxel_x / self.page_size,
            voxel_y / self.page_size,
            voxel_z / self.page_size,
        )
    }
    
    /// Get local voxel offset within a page
    pub fn voxel_offset_in_page(&self, voxel_x: u32, voxel_y: u32, voxel_z: u32) -> (u32, u32, u32) {
        (
            voxel_x % self.page_size,
            voxel_y % self.page_size,
            voxel_z % self.page_size,
        )
    }
}

/// Page table metadata for GPU access
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PageTableGpuHeader {
    /// World size in pages
    pub world_size_pages_x: u32,
    pub world_size_pages_y: u32,
    pub world_size_pages_z: u32,
    
    /// Page size in voxels
    pub page_size: u32,
    
    /// Total number of pages
    pub total_pages: u32,
    
    /// Number of resident pages
    pub resident_pages: u32,
    
    /// Padding for alignment
    pub _padding: [u32; 2],
}

/// Statistics for page table operations
#[derive(Debug, Default, Clone)]
pub struct PageTableStats {
    pub total_pages: u64,
    pub resident_pages: u32,
    pub dirty_pages: u32,
    pub locked_pages: u32,
    pub page_faults: u64,
    pub evictions: u64,
    pub compressions: u64,
    pub decompressions: u64,
}

/// Page eviction policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionPolicy {
    LRU,       // Least Recently Used
    LFU,       // Least Frequently Used
    FIFO,      // First In First Out
    Distance,  // Based on distance from camera
}

/// Create a page table for a world of given size
pub fn create_page_table(
    world_size_voxels: (u32, u32, u32),
    page_size: u32,
) -> PageTable {
    let world_size_pages = (
        (world_size_voxels.0 + page_size - 1) / page_size,
        (world_size_voxels.1 + page_size - 1) / page_size,
        (world_size_voxels.2 + page_size - 1) / page_size,
    );
    
    let total_pages = world_size_pages.0 as u64 * 
                     world_size_pages.1 as u64 * 
                     world_size_pages.2 as u64;
    
    // For very large worlds, use sparse representation
    let sparse_index = if total_pages > 1_000_000 {
        Some(create_sparse_index(&world_size_pages))
    } else {
        None
    };
    
    PageTable {
        entries: vec![PageTableEntry::empty(); total_pages.min(u32::MAX as u64) as usize],
        page_size,
        world_size_pages,
        total_pages,
        resident_pages: 0,
        sparse_index,
    }
}

/// Create sparse index for large worlds
fn create_sparse_index(world_size_pages: &(u32, u32, u32)) -> SparseIndex {
    // Calculate sizes for each level
    let level0_size = ((world_size_pages.0 + 7) / 8) *
                     ((world_size_pages.1 + 7) / 8) *
                     ((world_size_pages.2 + 7) / 8);
    
    let level1_size = ((world_size_pages.0 + 63) / 64) *
                     ((world_size_pages.1 + 63) / 64) *
                     ((world_size_pages.2 + 63) / 64);
    
    let level2_size = ((world_size_pages.0 + 511) / 512) *
                     ((world_size_pages.1 + 511) / 512) *
                     ((world_size_pages.2 + 511) / 512);
    
    SparseIndex {
        level0: vec![0; level0_size as usize],
        level1: vec![0; level1_size as usize],
        level2: vec![0; level2_size as usize],
    }
}