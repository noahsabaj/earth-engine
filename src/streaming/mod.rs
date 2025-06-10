// Data-oriented world streaming system
// Pure data structures for planet-scale voxel worlds

pub mod page_table;
pub mod morton_page_table;
pub mod memory_mapper;
pub mod gpu_vm;
pub mod predictive_loader;
pub mod stream_pipeline;
pub mod compression;

pub use page_table::{PageTable, PageTableEntry, PageFlags};
pub use morton_page_table::{MortonPageTable, MortonPageTableGpuHeader};
pub use memory_mapper::{MemoryMapper, MemorySegment};
pub use gpu_vm::{GpuVirtualMemory, GpuPageFault};
pub use predictive_loader::{PredictiveLoader, AccessPattern};
pub use stream_pipeline::{StreamPipeline, StreamRequest};
pub use compression::{CompressionType, GpuDecompressor};

/// Maximum supported world size (2^30 x 2^30 x 2^10 voxels)
pub const MAX_WORLD_SIZE_X: u32 = 1 << 30;
pub const MAX_WORLD_SIZE_Y: u32 = 1 << 10; 
pub const MAX_WORLD_SIZE_Z: u32 = 1 << 30;

/// Page size in voxels (64x64x64)
pub const PAGE_SIZE: u32 = 64;
pub const PAGE_VOXEL_COUNT: u32 = PAGE_SIZE * PAGE_SIZE * PAGE_SIZE;

/// Page size in bytes (assuming 4 bytes per voxel)
pub const PAGE_SIZE_BYTES: u64 = (PAGE_VOXEL_COUNT * 4) as u64;

/// Maximum pages in memory at once
pub const MAX_RESIDENT_PAGES: u32 = 16384; // ~4GB with 256KB pages