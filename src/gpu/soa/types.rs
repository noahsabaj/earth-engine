//! SOA type definitions and traits
//! 
//! This module defines the core types and traits for Structure of Arrays (SOA)
//! data layout, optimized for GPU memory access patterns.

use encase::{ShaderType, ShaderSize, internal::WriteInto};
use bytemuck::{Pod, Zeroable};
use crate::gpu::types::terrain::{BlockDistribution, TerrainParams, MAX_BLOCK_DISTRIBUTIONS};
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
    pub _pad: [u32; 3],
    
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

impl Default for BlockDistributionSOA {
    fn default() -> Self {
        Self {
            count: 0,
            _pad: [0; 3],
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
            // Default padding
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
            _pad4: 0,
            _pad5: 0,
            _pad6: 0,
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
    pub _pad: [u32; 2],
    
    // Embedded SOA distributions for cache-friendly access
    pub distributions: BlockDistributionSOA,
}

impl Default for TerrainParamsSOA {
    fn default() -> Self {
        Self {
            seed: 12345,
            sea_level: 64.0,
            terrain_scale: 0.01,
            mountain_threshold: 0.6,
            cave_threshold: 0.3,
            num_distributions: 0,
            _pad: [0; 2],
            distributions: BlockDistributionSOA::default(),
        }
    }
}

impl TerrainParamsSOA {
    /// Create from existing TerrainParams (AOS format)
    pub fn from_aos(params: &TerrainParams) -> Self {
        let distributions = BlockDistribution::to_soa(
            &params.distributions[..params.num_distributions as usize]
        );
        
        Self {
            seed: params.seed,
            sea_level: params.sea_level,
            terrain_scale: params.terrain_scale,
            mountain_threshold: params.mountain_threshold,
            cave_threshold: params.cave_threshold,
            num_distributions: params.num_distributions,
            _pad: params._padding,
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
        params._padding = self._pad;
        
        // Convert distributions back
        for i in 0..self.distributions.count as usize {
            params.distributions[i] = BlockDistribution::from_soa(&self.distributions, i);
        }
        
        params
    }
}

/// Compile-time validation of SOA sizes
#[cfg(debug_assertions)]
pub fn validate_soa_sizes() {
    use encase::ShaderSize;
    
    let block_soa_size = BlockDistributionSOA::SHADER_SIZE.get();
    let terrain_soa_size = TerrainParamsSOA::SHADER_SIZE.get();
    
    log::info!("[SOA Types] BlockDistributionSOA size: {} bytes", block_soa_size);
    log::info!("[SOA Types] TerrainParamsSOA size: {} bytes", terrain_soa_size);
    
    // Verify alignment
    assert!(block_soa_size % 16 == 0, "BlockDistributionSOA must be 16-byte aligned");
    assert!(terrain_soa_size % 16 == 0, "TerrainParamsSOA must be 16-byte aligned");
}