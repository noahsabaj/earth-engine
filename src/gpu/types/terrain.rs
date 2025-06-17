//! GPU types for terrain generation

use encase::ShaderType;
use bytemuck::{Pod, Zeroable};
use crate::gpu::types::core::GpuData;

/// Maximum number of custom block distributions
/// This is a GPU limitation - we need fixed-size arrays in shaders
pub const MAX_BLOCK_DISTRIBUTIONS: usize = 16;

/// Generic block distribution rule for GPU terrain generation
/// 
/// This struct is automatically aligned to 48 bytes for GPU compatibility
#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone, Debug)]
pub struct BlockDistribution {
    /// Block ID to place
    pub block_id: u32,
    /// Minimum Y coordinate (inclusive)
    pub min_height: i32,
    /// Maximum Y coordinate (inclusive)
    pub max_height: i32,
    /// Base probability (0.0-1.0)
    pub probability: f32,
    /// Noise threshold for placement (0.0-1.0)
    /// Used for clustering - higher values create more sparse placement
    pub noise_threshold: f32,
    /// Padding to reach 48 bytes for GPU alignment
    /// Must match the WGSL struct definition
    pub _padding: [u32; 7], // 28 bytes to reach 48 total
}

impl Default for BlockDistribution {
    fn default() -> Self {
        Self {
            block_id: 0,
            min_height: i32::MIN,
            max_height: i32::MAX,
            probability: 0.0,
            noise_threshold: 0.5,
            _padding: [0; 7],
        }
    }
}

/// Parameters for GPU terrain generation
#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone)]
pub struct TerrainParams {
    /// World seed for deterministic generation
    pub seed: u32,
    /// Sea level height
    pub sea_level: f32,
    /// Base terrain scale
    pub terrain_scale: f32,
    /// Mountain threshold
    pub mountain_threshold: f32,
    /// Cave density threshold
    pub cave_threshold: f32,
    /// Number of active block distributions (0 to MAX_BLOCK_DISTRIBUTIONS)
    pub num_distributions: u32,
    /// Padding for alignment
    pub _padding: [u32; 2],
    /// Custom block distributions
    /// Games can specify up to MAX_BLOCK_DISTRIBUTIONS custom blocks
    pub distributions: [BlockDistribution; MAX_BLOCK_DISTRIBUTIONS],
}

impl Default for TerrainParams {
    fn default() -> Self {
        Self {
            seed: 12345,
            sea_level: 64.0,
            terrain_scale: 0.01,
            mountain_threshold: 0.6,
            cave_threshold: 0.3,
            num_distributions: 0,
            _padding: [0; 2],
            distributions: [BlockDistribution::default(); MAX_BLOCK_DISTRIBUTIONS],
        }
    }
}

impl TerrainParams {
    /// Add a block distribution rule
    /// Returns true if added, false if at capacity
    pub fn add_distribution(&mut self, distribution: BlockDistribution) -> bool {
        if self.num_distributions as usize >= MAX_BLOCK_DISTRIBUTIONS {
            log::warn!("[TerrainParams] Cannot add distribution - at maximum capacity ({} distributions)", MAX_BLOCK_DISTRIBUTIONS);
            return false;
        }
        
        let index = self.num_distributions as usize;
        self.distributions[index] = distribution;
        self.num_distributions += 1;
        
        log::debug!("[TerrainParams] Added distribution for block {} at index {} (total: {})", 
                   distribution.block_id, index, self.num_distributions);
        true
    }
    
    /// Clear all distributions
    pub fn clear_distributions(&mut self) {
        self.num_distributions = 0;
        // Zero out for safety
        self.distributions = [BlockDistribution::default(); MAX_BLOCK_DISTRIBUTIONS];
    }
}

// Compile-time size validation
// Note: encase handles padding automatically, so we can't predict exact sizes
// We'll validate these at runtime in debug builds instead
#[cfg(debug_assertions)]
pub fn validate_terrain_sizes() {
    use encase::ShaderSize;
    
    let block_size = BlockDistribution::SHADER_SIZE.get();
    let params_size = TerrainParams::SHADER_SIZE.get();
    
    log::info!("[GPU Types] BlockDistribution size: {} bytes", block_size);
    log::info!("[GPU Types] TerrainParams size: {} bytes", params_size);
    
    // Verify alignment
    assert!(block_size % 16 == 0, "BlockDistribution must be 16-byte aligned");
    assert!(params_size % 16 == 0, "TerrainParams must be 16-byte aligned");
}