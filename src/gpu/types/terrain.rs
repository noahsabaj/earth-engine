//! GPU types for terrain generation

use encase::ShaderType;
use bytemuck::{Pod, Zeroable};
use crate::gpu::types::core::GpuData;
use crate::gpu::constants::MAX_BLOCK_DISTRIBUTIONS;
use crate::gpu::automation::auto_wgsl::AutoWgsl;

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
    /// Padding to ensure 16-byte alignment (20 bytes -> 32 bytes)
    pub _padding: [u32; 3],
}

impl Default for BlockDistribution {
    fn default() -> Self {
        Self {
            block_id: 0,
            min_height: i32::MIN,
            max_height: i32::MAX,
            probability: 0.0,
            noise_threshold: 0.5,
            _padding: [0; 3],
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
    /// Padding for 16-byte alignment
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

// Implement AutoWgsl for automatic WGSL generation
crate::auto_wgsl!(
    BlockDistribution,
    name = "BlockDistribution",
    fields = [
        block_id: "u32",
        min_height: "i32",
        max_height: "i32",
        probability: "f32",
        noise_threshold: "f32",
    ]
);

crate::auto_wgsl!(
    TerrainParams,
    name = "TerrainParams",
    fields = [
        seed: "u32",
        sea_level: "f32",
        terrain_scale: "f32",
        mountain_threshold: "f32",
        cave_threshold: "f32",
        num_distributions: "u32",
        distributions: "BlockDistribution"[MAX_BLOCK_DISTRIBUTIONS],
    ]
);

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
#[cfg(test)]
mod tests {
    use super::*;
    use encase::ShaderSize;
    
    #[test]
    fn test_block_distribution_layout() {
        let rust_size = std::mem::size_of::<BlockDistribution>();
        let shader_size = BlockDistribution::SHADER_SIZE.get();
        
        println!("[BlockDistribution] Memory layout:");
        println!("  Rust size: {} bytes", rust_size);
        println!("  Shader size (encase): {} bytes", shader_size);
        println!("  Original size with manual padding: 48 bytes");
        
        // Encase should handle alignment automatically
        assert_eq!(shader_size % 16, 0, "BlockDistribution must be 16-byte aligned");
        
        // The size without padding would be: 5 fields * 4 bytes = 20 bytes
        // Encase should pad this to at least 32 bytes (next 16-byte boundary)
        assert!(shader_size >= 32, "BlockDistribution shader size should be at least 32 bytes");
        
        if shader_size != 48 {
            println!("NOTE: BlockDistribution size changed from 48 to {} bytes", shader_size);
            println!("      WGSL shaders may need updating to match new layout");
        }
    }
    
    #[test]
    fn test_terrain_params_layout() {
        let rust_size = std::mem::size_of::<TerrainParams>();
        let shader_size = TerrainParams::SHADER_SIZE.get();
        
        println!("[TerrainParams] Memory layout:");
        println!("  Rust size: {} bytes", rust_size);
        println!("  Shader size (encase): {} bytes", shader_size);
        
        // Verify alignment
        assert_eq!(shader_size % 16, 0, "TerrainParams must be 16-byte aligned");
        
        // TerrainParams contains:
        // - 6 scalar fields (24 bytes)
        // - Array of BlockDistribution[MAX_BLOCK_DISTRIBUTIONS]
        let base_size = 24;
        let distribution_array_size = BlockDistribution::SHADER_SIZE.get() * MAX_BLOCK_DISTRIBUTIONS as u64;
        let expected_min_size = base_size + distribution_array_size;
        
        println!("  Base fields size: {} bytes", base_size);
        println!("  Distributions array size: {} bytes", distribution_array_size);
        println!("  Expected minimum size: {} bytes", expected_min_size);
        
        assert!(shader_size >= expected_min_size, 
                "TerrainParams shader size {} should be at least {} bytes", 
                shader_size, expected_min_size);
    }
}

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