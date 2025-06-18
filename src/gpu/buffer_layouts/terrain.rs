//! Terrain generation buffer layout definitions
//! 
//! Defines GPU buffer structures for terrain generation parameters.

use bytemuck::{Pod, Zeroable};
use encase::ShaderType;

/// Block distribution parameters for terrain generation
/// Defines how a specific block type appears in the world
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, ShaderType)]
pub struct BlockDistribution {
    /// Block ID to place
    pub block_id: u32,
    
    /// Minimum Y level (inclusive)
    pub min_y: i32,
    
    /// Maximum Y level (inclusive)
    pub max_y: i32,
    
    /// Noise threshold for placement (0.0 to 1.0)
    pub threshold: f32,
    
    /// Noise scale factor
    pub scale: f32,
    
    /// Octaves for fractal noise
    pub octaves: u32,
    
    /// Persistence for fractal noise
    pub persistence: f32,
    
    /// Lacunarity for fractal noise
    pub lacunarity: f32,
}

impl Default for BlockDistribution {
    fn default() -> Self {
        Self {
            block_id: 0,
            min_y: 0,
            max_y: 255,
            threshold: 0.5,
            scale: 1.0,
            octaves: 4,
            persistence: 0.5,
            lacunarity: 2.0,
        }
    }
}

/// Terrain generation parameters (Array of Structures layout)
/// Used for CPU-side terrain generation and simple GPU kernels
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, ShaderType)]
pub struct TerrainParams {
    /// Random seed for generation
    pub seed: u32,
    
    /// Sea level height
    pub sea_level: i32,
    
    /// Maximum terrain height
    pub max_height: i32,
    
    /// Base terrain scale
    pub terrain_scale: f32,
    
    /// Height scale multiplier
    pub height_scale: f32,
    
    /// Number of active block distributions
    pub distribution_count: u32,
    
    /// Padding for alignment
    pub _padding: [u32; 2],
    
    /// Block distribution parameters (up to 16)
    pub distributions: [BlockDistribution; 16],
}

impl Default for TerrainParams {
    fn default() -> Self {
        let mut distributions = [BlockDistribution::default(); 16];
        
        // Default terrain layers
        distributions[0] = BlockDistribution {
            block_id: 1, // Stone
            min_y: 0,
            max_y: 64,
            threshold: 0.0,
            scale: 1.0,
            octaves: 1,
            persistence: 0.5,
            lacunarity: 2.0,
        };
        
        distributions[1] = BlockDistribution {
            block_id: 2, // Dirt
            min_y: 60,
            max_y: 70,
            threshold: 0.3,
            scale: 0.02,
            octaves: 3,
            persistence: 0.5,
            lacunarity: 2.0,
        };
        
        distributions[2] = BlockDistribution {
            block_id: 3, // Grass
            min_y: 65,
            max_y: 72,
            threshold: 0.4,
            scale: 0.01,
            octaves: 2,
            persistence: 0.6,
            lacunarity: 2.0,
        };
        
        Self {
            seed: 12345,
            sea_level: 64,
            max_height: 128,
            terrain_scale: 0.01,
            height_scale: 64.0,
            distribution_count: 3,
            _padding: [0; 2],
            distributions,
        }
    }
}

/// Terrain generation parameters (Structure of Arrays layout)
/// Optimized for GPU memory access patterns
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TerrainParamsSOA {
    /// Base parameters
    pub seed: u32,
    pub sea_level: i32,
    pub max_height: i32,
    pub terrain_scale: f32,
    pub height_scale: f32,
    pub distribution_count: u32,
    pub _padding: [u32; 2],
    
    /// Arrays for each field (SOA layout)
    pub block_ids: [u32; 16],
    pub min_y_values: [i32; 16],
    pub max_y_values: [i32; 16],
    pub thresholds: [f32; 16],
    pub scales: [f32; 16],
    pub octaves: [u32; 16],
    pub persistence_values: [f32; 16],
    pub lacunarity_values: [f32; 16],
}

impl TerrainParamsSOA {
    /// Convert from AOS to SOA layout
    pub fn from_aos(params: &TerrainParams) -> Self {
        let mut soa = Self {
            seed: params.seed,
            sea_level: params.sea_level,
            max_height: params.max_height,
            terrain_scale: params.terrain_scale,
            height_scale: params.height_scale,
            distribution_count: params.distribution_count,
            _padding: [0; 2],
            block_ids: [0; 16],
            min_y_values: [0; 16],
            max_y_values: [0; 16],
            thresholds: [0.0; 16],
            scales: [0.0; 16],
            octaves: [0; 16],
            persistence_values: [0.0; 16],
            lacunarity_values: [0.0; 16],
        };
        
        // Transpose the data
        for (i, dist) in params.distributions.iter().enumerate() {
            soa.block_ids[i] = dist.block_id;
            soa.min_y_values[i] = dist.min_y;
            soa.max_y_values[i] = dist.max_y;
            soa.thresholds[i] = dist.threshold;
            soa.scales[i] = dist.scale;
            soa.octaves[i] = dist.octaves;
            soa.persistence_values[i] = dist.persistence;
            soa.lacunarity_values[i] = dist.lacunarity;
        }
        
        soa
    }
    
    /// Convert from SOA back to AOS layout
    pub fn to_aos(&self) -> TerrainParams {
        let mut params = TerrainParams {
            seed: self.seed,
            sea_level: self.sea_level,
            max_height: self.max_height,
            terrain_scale: self.terrain_scale,
            height_scale: self.height_scale,
            distribution_count: self.distribution_count,
            _padding: [0; 2],
            distributions: [BlockDistribution::default(); 16],
        };
        
        // Transpose the data back
        for i in 0..16 {
            params.distributions[i] = BlockDistribution {
                block_id: self.block_ids[i],
                min_y: self.min_y_values[i],
                max_y: self.max_y_values[i],
                threshold: self.thresholds[i],
                scale: self.scales[i],
                octaves: self.octaves[i],
                persistence: self.persistence_values[i],
                lacunarity: self.lacunarity_values[i],
            };
        }
        
        params
    }
}

/// Biome parameters for advanced terrain generation
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct BiomeParams {
    /// Temperature range (min, max)
    pub temperature_range: [f32; 2],
    
    /// Humidity range (min, max)
    pub humidity_range: [f32; 2],
    
    /// Primary block distributions for this biome
    pub primary_blocks: [u32; 4],
    
    /// Biome blend factor
    pub blend_factor: f32,
    
    /// Padding
    pub _padding: [f32; 3],
}

/// Terrain buffer layout information
pub struct TerrainBufferLayout;

impl TerrainBufferLayout {
    /// Size of AOS terrain parameters
    pub const PARAMS_SIZE: u64 = std::mem::size_of::<TerrainParams>() as u64;
    
    /// Size of SOA terrain parameters
    pub const PARAMS_SOA_SIZE: u64 = std::mem::size_of::<TerrainParamsSOA>() as u64;
    
    /// Size of biome parameters
    pub const BIOME_SIZE: u64 = std::mem::size_of::<BiomeParams>() as u64;
}