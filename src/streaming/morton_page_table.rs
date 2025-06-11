use bytemuck::{Pod, Zeroable};
use crate::morton::{morton_encode, morton_decode};
use super::page_table::{PageTableEntry, PageFlags, SparseIndex};

/// Page table using Morton encoding for improved cache locality
/// 
/// Pages are stored in Morton order, which dramatically improves
/// cache performance when accessing neighboring pages during
/// world streaming and traversal.
#[repr(C)]
#[derive(Debug)]
pub struct MortonPageTable {
    /// Flat array of page entries in Morton order
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

impl MortonPageTable {
    /// Create a new Morton-encoded page table
    pub fn new(world_size_pages: (u32, u32, u32), page_size: u32) -> Self {
        let total_pages = world_size_pages.0 as u64 * 
                         world_size_pages.1 as u64 * 
                         world_size_pages.2 as u64;
        
        Self {
            entries: vec![PageTableEntry::empty(); total_pages as usize],
            page_size,
            world_size_pages,
            total_pages,
            resident_pages: 0,
            sparse_index: None,
        }
    }
    
    /// Calculate Morton-encoded page index from page coordinates
    pub fn page_index(&self, page_x: u32, page_y: u32, page_z: u32) -> Option<usize> {
        if page_x >= self.world_size_pages.0 ||
           page_y >= self.world_size_pages.1 ||
           page_z >= self.world_size_pages.2 {
            return None;
        }
        
        // Use Morton encoding for page index
        let morton = morton_encode(page_x, page_y, page_z);
        
        // For small worlds, Morton code directly maps to index
        // For large worlds, we'd need a hash table
        if morton < self.total_pages {
            Some(morton as usize)
        } else {
            // This shouldn't happen with proper bounds checking
            None
        }
    }
    
    /// Get page coordinates from Morton index
    pub fn index_to_page(&self, index: usize) -> (u32, u32, u32) {
        morton_decode(index as u64)
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
    
    /// Get Morton index for a voxel position
    pub fn voxel_to_morton_index(&self, voxel_x: u32, voxel_y: u32, voxel_z: u32) -> Option<usize> {
        let (page_x, page_y, page_z) = self.voxel_to_page(voxel_x, voxel_y, voxel_z);
        self.page_index(page_x, page_y, page_z)
    }
    
    /// Iterate pages in Morton order for cache-friendly traversal
    pub fn iter_pages_morton(&self) -> MortonPageIterator {
        MortonPageIterator {
            table: self,
            current_index: 0,
        }
    }
    
    /// Get neighboring pages in Morton order (for prefetching)
    pub fn get_neighbor_pages(&self, page_x: u32, page_y: u32, page_z: u32) -> Vec<usize> {
        let mut neighbors = Vec::with_capacity(27);
        
        for dx in -1i32..=1 {
            for dy in -1i32..=1 {
                for dz in -1i32..=1 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue; // Skip center
                    }
                    
                    let nx = page_x as i32 + dx;
                    let ny = page_y as i32 + dy;
                    let nz = page_z as i32 + dz;
                    
                    if nx >= 0 && ny >= 0 && nz >= 0 {
                        if let Some(idx) = self.page_index(nx as u32, ny as u32, nz as u32) {
                            neighbors.push(idx);
                        }
                    }
                }
            }
        }
        
        // Sort by Morton code to improve cache access
        neighbors.sort_unstable();
        neighbors
    }
    
    /// Mark a page as accessed (for LRU tracking)
    pub fn mark_accessed(&mut self, page_x: u32, page_y: u32, page_z: u32) {
        if let Some(idx) = self.page_index(page_x, page_y, page_z) {
            if let Some(entry) = self.entries.get_mut(idx) {
                entry.access_count = entry.access_count.saturating_add(1);
            }
        }
    }
    
    /// Find least recently used pages for eviction
    pub fn find_lru_pages(&self, count: usize) -> Vec<usize> {
        let mut pages: Vec<(usize, u16)> = self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.is_resident() && !entry.is_locked())
            .map(|(idx, entry)| (idx, entry.access_count))
            .collect();
        
        pages.sort_unstable_by_key(|(_, access_count)| *access_count);
        pages.into_iter()
            .take(count)
            .map(|(idx, _)| idx)
            .collect()
    }
}

/// Iterator for Morton-ordered page traversal
pub struct MortonPageIterator<'a> {
    table: &'a MortonPageTable,
    current_index: usize,
}

impl<'a> Iterator for MortonPageIterator<'a> {
    type Item = (u32, u32, u32, &'a PageTableEntry);
    
    fn next(&mut self) -> Option<Self::Item> {
        while self.current_index < self.table.entries.len() {
            let entry = &self.table.entries[self.current_index];
            let (x, y, z) = self.table.index_to_page(self.current_index);
            self.current_index += 1;
            
            // Only return non-empty pages
            if entry.flags != PageFlags::Empty as u8 {
                return Some((x, y, z, entry));
            }
        }
        None
    }
}

/// GPU header for Morton page table
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MortonPageTableGpuHeader {
    /// World size in pages
    pub world_size_pages_x: u32,
    pub world_size_pages_y: u32,
    pub world_size_pages_z: u32,
    
    /// Page size in voxels
    pub page_size: u32,
    
    /// Total pages
    pub total_pages: u32,
    
    /// Currently resident pages
    pub resident_pages: u32,
    
    /// Padding for alignment
    pub _padding: [u32; 2],
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_morton_page_table() {
        let table = MortonPageTable::new((16, 16, 16), 64);
        
        // Test page indexing
        assert_eq!(table.page_index(0, 0, 0), Some(0));
        
        // Test voxel to page conversion
        assert_eq!(table.voxel_to_page(127, 127, 127), (1, 1, 1));
        
        // Test neighbor finding
        let neighbors = table.get_neighbor_pages(1, 1, 1);
        assert_eq!(neighbors.len(), 26); // 3x3x3 - 1 (center)
    }
}