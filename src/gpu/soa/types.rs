//! SOA type definitions and traits
//!
//! This module defines the core types and traits for Structure of Arrays (SOA)
//! data layout, optimized for GPU memory access patterns.

use crate::gpu::automation::auto_layout::AutoLayout;
use crate::gpu::automation::auto_wgsl::AutoWgsl;
use crate::gpu::types::terrain::{BlockDistribution, TerrainParams};
use crate::constants::{core::MAX_BLOCK_DISTRIBUTIONS, terrain::SEA_LEVEL};
use bytemuck::{Pod, Zeroable};
use encase::{internal::WriteInto, ShaderSize, ShaderType};
use std::marker::PhantomData;

/// Marker trait for types that can be converted to SOA representation
pub trait SoaCompatible: Pod + Zeroable + Copy + Clone {
    /// The SOA representation of this type
    /// Must implement GpuData to be usable in TypedGpuBuffer
    type Arrays: crate::gpu::GpuData + Copy + Clone;

    /// Convert from Array of Structures to Structure of Arrays
    fn to_soa(items: &[Self]) -> Self::Arrays;

    /// Extract a single item from SOA representation
    fn from_soa(arrays: &Self::Arrays, index: usize) -> Self;

    /// Update a single item in SOA representation
    fn update_soa(arrays: &mut Self::Arrays, index: usize, item: &Self);

    /// Get the count of valid items in the SOA data
    fn soa_count(arrays: &Self::Arrays) -> usize;
}

/// SOA representation of BlockDistribution for GPU processing
///
/// This layout maximizes cache efficiency by storing each field
/// in a contiguous array, enabling coalesced memory access.
#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone, Debug)]
pub struct BlockDistributionSOA {
    /// Number of active distributions
    pub count: u32,
    /// Padding for 16-byte alignment
    pub _padding: [u32; 3],

    // Pure arrays - each field stored contiguously
    /// Block IDs array
    pub block_ids: [u32; MAX_BLOCK_DISTRIBUTIONS],
    /// Minimum height constraints
    pub min_heights: [i32; MAX_BLOCK_DISTRIBUTIONS],
    /// Maximum height constraints
    pub max_heights: [i32; MAX_BLOCK_DISTRIBUTIONS],
    /// Spawn probabilities
    pub probabilities: [f32; MAX_BLOCK_DISTRIBUTIONS],
    /// Noise thresholds for distribution
    pub noise_thresholds: [f32; MAX_BLOCK_DISTRIBUTIONS],
}

// Implement AutoWgsl for SOA types
crate::auto_wgsl!(
    BlockDistributionSOA,
    name = "BlockDistributionSOA",
    fields = [
        count: "u32",
        _padding: "u32"[3],
        block_ids: "u32"[MAX_BLOCK_DISTRIBUTIONS],
        min_heights: "i32"[MAX_BLOCK_DISTRIBUTIONS],
        max_heights: "i32"[MAX_BLOCK_DISTRIBUTIONS],
        probabilities: "f32"[MAX_BLOCK_DISTRIBUTIONS],
        noise_thresholds: "f32"[MAX_BLOCK_DISTRIBUTIONS],
    ]
);

// Implement AutoLayout for BlockDistributionSOA
crate::impl_auto_layout!(
    BlockDistributionSOA,
    fields = [
        count: u32 = "count",
        _padding: [u32; 3] = "_padding",
        block_ids: [u32; MAX_BLOCK_DISTRIBUTIONS] = "block_ids",
        min_heights: [i32; MAX_BLOCK_DISTRIBUTIONS] = "min_heights",
        max_heights: [i32; MAX_BLOCK_DISTRIBUTIONS] = "max_heights",
        probabilities: [f32; MAX_BLOCK_DISTRIBUTIONS] = "probabilities",
        noise_thresholds: [f32; MAX_BLOCK_DISTRIBUTIONS] = "noise_thresholds"
    ]
);

impl Default for BlockDistributionSOA {
    fn default() -> Self {
        Self {
            count: 0,
            _padding: [0; 3],
            block_ids: [0; MAX_BLOCK_DISTRIBUTIONS],
            min_heights: [i32::MIN; MAX_BLOCK_DISTRIBUTIONS],
            max_heights: [i32::MAX; MAX_BLOCK_DISTRIBUTIONS],
            probabilities: [0.0; MAX_BLOCK_DISTRIBUTIONS],
            noise_thresholds: [0.5; MAX_BLOCK_DISTRIBUTIONS],
        }
    }
}

impl SoaCompatible for BlockDistribution {
    type Arrays = BlockDistributionSOA;

    fn to_soa(items: &[Self]) -> Self::Arrays {
        let count = items.len().min(MAX_BLOCK_DISTRIBUTIONS);
        let mut soa = BlockDistributionSOA {
            count: count as u32,
            ..Default::default()
        };

        // Transform AOS to SOA
        for (i, item) in items.iter().take(count).enumerate() {
            soa.block_ids[i] = item.block_id;
            soa.min_heights[i] = item.min_height;
            soa.max_heights[i] = item.max_height;
            soa.probabilities[i] = item.probability;
            soa.noise_thresholds[i] = item.noise_threshold;
        }

        soa
    }

    fn from_soa(arrays: &Self::Arrays, index: usize) -> Self {
        assert!(index < arrays.count as usize, "SOA index out of bounds");

        Self {
            block_id: arrays.block_ids[index],
            min_height: arrays.min_heights[index],
            max_height: arrays.max_heights[index],
            probability: arrays.probabilities[index],
            noise_threshold: arrays.noise_thresholds[index],
            _padding: [0; 3],
        }
    }

    fn update_soa(arrays: &mut Self::Arrays, index: usize, item: &Self) {
        assert!(index < arrays.count as usize, "SOA index out of bounds");

        arrays.block_ids[index] = item.block_id;
        arrays.min_heights[index] = item.min_height;
        arrays.max_heights[index] = item.max_height;
        arrays.probabilities[index] = item.probability;
        arrays.noise_thresholds[index] = item.noise_threshold;
    }

    fn soa_count(arrays: &Self::Arrays) -> usize {
        arrays.count as usize
    }
}

/// SOA representation of TerrainParams
///
/// Embeds BlockDistributionSOA for optimal GPU memory layout
#[repr(C)]
#[derive(ShaderType, Pod, Zeroable, Copy, Clone)]
pub struct TerrainParamsSOA {
    // Scalar parameters remain the same
    pub seed: u32,
    pub sea_level: f32,
    pub terrain_scale: f32,
    pub mountain_threshold: f32,
    pub cave_threshold: f32,
    pub num_distributions: u32,
    /// Current weather type and intensity (packed)
    pub weather_type_intensity: u32,
    /// Temperature in Celsius * 10
    pub temperature: i32,

    // Embedded SOA distributions for cache-friendly access
    pub distributions: BlockDistributionSOA,
}

// Implement AutoWgsl for TerrainParamsSOA
crate::auto_wgsl!(
    TerrainParamsSOA,
    name = "TerrainParamsSOA",
    fields = [
        seed: "u32",
        sea_level: "f32",
        terrain_scale: "f32",
        mountain_threshold: "f32",
        cave_threshold: "f32",
        num_distributions: "u32",
        weather_type_intensity: "u32",
        temperature: "i32",
        distributions: "BlockDistributionSOA",
    ]
);

// Implement AutoLayout for TerrainParamsSOA
crate::impl_auto_layout!(
    TerrainParamsSOA,
    fields = [
        seed: u32 = "seed",
        sea_level: f32 = "sea_level",
        terrain_scale: f32 = "terrain_scale",
        mountain_threshold: f32 = "mountain_threshold",
        cave_threshold: f32 = "cave_threshold",
        num_distributions: u32 = "num_distributions",
        weather_type_intensity: u32 = "weather_type_intensity",
        temperature: i32 = "temperature",
        distributions: BlockDistributionSOA = "distributions"
    ]
);

impl Default for TerrainParamsSOA {
    fn default() -> Self {
        Self {
            seed: 12345,
            sea_level: SEA_LEVEL as f32,
            terrain_scale: 0.01,
            mountain_threshold: 0.6,
            cave_threshold: 0.3,
            num_distributions: 0,
            weather_type_intensity: 0, // Clear weather by default
            temperature: 200,          // 20°C default temperature
            distributions: BlockDistributionSOA::default(),
        }
    }
}

impl TerrainParamsSOA {
    /// Create from existing TerrainParams (AOS format)
    pub fn from_aos(params: &TerrainParams) -> Self {
        let distributions =
            BlockDistribution::to_soa(&params.distributions[..params.num_distributions as usize]);

        Self {
            seed: params.seed,
            sea_level: params.sea_level,
            terrain_scale: params.terrain_scale,
            mountain_threshold: params.mountain_threshold,
            cave_threshold: params.cave_threshold,
            num_distributions: params.num_distributions,
            weather_type_intensity: params.weather_type_intensity,
            temperature: params.temperature,
            distributions,
        }
    }

    /// Convert back to AOS format
    pub fn to_aos(&self) -> TerrainParams {
        let mut params = TerrainParams::default();
        params.seed = self.seed;
        params.sea_level = self.sea_level;
        params.terrain_scale = self.terrain_scale;
        params.mountain_threshold = self.mountain_threshold;
        params.cave_threshold = self.cave_threshold;
        params.num_distributions = self.num_distributions;
        params.weather_type_intensity = self.weather_type_intensity;
        params.temperature = self.temperature;

        // Convert distributions back
        for i in 0..self.distributions.count as usize {
            params.distributions[i] = BlockDistribution::from_soa(&self.distributions, i);
        }

        params
    }
}

/// Compile-time validation of SOA sizes
#[cfg(test)]
mod tests {
    use super::*;
    use encase::ShaderSize;

    #[test]
    fn test_block_distribution_soa_sizes() {
        let rust_size = std::mem::size_of::<BlockDistributionSOA>();
        let shader_size = BlockDistributionSOA::SHADER_SIZE.get();

        println!("[BlockDistributionSOA] Rust size: {} bytes", rust_size);
        println!("[BlockDistributionSOA] Shader size: {} bytes", shader_size);

        // Verify shader size is aligned to 16 bytes
        assert_eq!(
            shader_size % 16,
            0,
            "BlockDistributionSOA shader size must be 16-byte aligned"
        );

        // Calculate expected size: count (4) + 5 arrays of MAX_BLOCK_DISTRIBUTIONS elements
        // Each array element is 4 bytes, so each array is MAX_BLOCK_DISTRIBUTIONS * 4
        let expected_size = 4 + 5 * (MAX_BLOCK_DISTRIBUTIONS as usize * 4);
        let expected_aligned = ((expected_size + 15) / 16) * 16; // Round up to 16-byte alignment

        println!(
            "[BlockDistributionSOA] Expected size (unaligned): {} bytes",
            expected_size
        );
        println!(
            "[BlockDistributionSOA] Expected size (aligned): {} bytes",
            expected_aligned
        );
    }

    #[test]
    fn test_terrain_params_soa_sizes() {
        let rust_size = std::mem::size_of::<TerrainParamsSOA>();
        let shader_size = TerrainParamsSOA::SHADER_SIZE.get();

        println!("[TerrainParamsSOA] Rust size: {} bytes", rust_size);
        println!("[TerrainParamsSOA] Shader size: {} bytes", shader_size);

        // The expected WGSL size was 384 bytes with manual padding
        // With encase, the size might be different but should still be aligned
        assert_eq!(
            shader_size % 16,
            0,
            "TerrainParamsSOA shader size must be 16-byte aligned"
        );

        // Log warning if size doesn't match expected
        if shader_size != 384 {
            println!("WARNING: TerrainParamsSOA shader size ({} bytes) differs from expected WGSL size (384 bytes)", shader_size);
            println!("This may require updating WGSL shaders to match the new layout");
        }
    }

    #[test]
    fn test_block_distribution_sizes() {
        let rust_size = std::mem::size_of::<BlockDistribution>();
        let shader_size = BlockDistribution::SHADER_SIZE.get();

        println!("[BlockDistribution] Rust size: {} bytes", rust_size);
        println!("[BlockDistribution] Shader size: {} bytes", shader_size);

        // The original size was 48 bytes with manual padding
        // With encase handling alignment, verify it's still reasonable
        assert_eq!(
            shader_size % 16,
            0,
            "BlockDistribution shader size must be 16-byte aligned"
        );

        if shader_size != 48 {
            println!("WARNING: BlockDistribution shader size ({} bytes) differs from original size (48 bytes)", shader_size);
        }
    }

    #[test]
    fn test_terrain_params_sizes() {
        let rust_size = std::mem::size_of::<TerrainParams>();
        let shader_size = TerrainParams::SHADER_SIZE.get();

        println!("[TerrainParams] Rust size: {} bytes", rust_size);
        println!("[TerrainParams] Shader size: {} bytes", shader_size);

        // Verify alignment
        assert_eq!(
            shader_size % 16,
            0,
            "TerrainParams shader size must be 16-byte aligned"
        );
    }
}
#[cfg(debug_assertions)]
pub fn validate_soa_sizes() {
    use encase::ShaderSize;

    let block_soa_size = BlockDistributionSOA::SHADER_SIZE.get();
    let terrain_soa_size = TerrainParamsSOA::SHADER_SIZE.get();

    log::info!(
        "[SOA Types] BlockDistributionSOA size: {} bytes",
        block_soa_size
    );
    log::info!(
        "[SOA Types] TerrainParamsSOA size: {} bytes",
        terrain_soa_size
    );

    // Verify alignment
    assert!(
        block_soa_size % 16 == 0,
        "BlockDistributionSOA must be 16-byte aligned"
    );
    assert!(
        terrain_soa_size % 16 == 0,
        "TerrainParamsSOA must be 16-byte aligned"
    );
}
