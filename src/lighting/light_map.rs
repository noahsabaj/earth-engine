use crate::world::VoxelPos;

/// Light level (0-15) with separate sky and block light components
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightLevel {
    /// Skylight level (0-15)
    pub sky: u8,
    /// Block light level (0-15)
    pub block: u8,
}

impl LightLevel {
    pub fn new(sky: u8, block: u8) -> Self {
        Self {
            sky: sky.min(15),
            block: block.min(15),
        }
    }
    
    /// Get the maximum light level from either source
    pub fn max_light(&self) -> u8 {
        self.sky.max(self.block)
    }
    
    /// Get combined light level for rendering
    pub fn combined(&self) -> u8 {
        self.sky.max(self.block)
    }
    
    /// Create a dark light level
    pub fn dark() -> Self {
        Self { sky: 0, block: 0 }
    }
    
    /// Create a fully lit skylight level
    pub fn full_sky() -> Self {
        Self { sky: 15, block: 0 }
    }
}

/// Light storage for a chunk
pub struct LightMap {
    size: u32,
    /// Packed light data: sky light in upper 4 bits, block light in lower 4 bits
    data: Vec<u8>,
}

impl LightMap {
    pub fn new(size: u32) -> Self {
        let total_size = (size * size * size) as usize;
        Self {
            size,
            data: vec![0; total_size],
        }
    }
    
    fn index(&self, x: u32, y: u32, z: u32) -> usize {
        (y * self.size * self.size + z * self.size + x) as usize
    }
    
    pub fn get_light(&self, x: u32, y: u32, z: u32) -> LightLevel {
        if x >= self.size || y >= self.size || z >= self.size {
            return LightLevel::dark();
        }
        
        let packed = self.data[self.index(x, y, z)];
        LightLevel {
            sky: (packed >> 4) & 0x0F,
            block: packed & 0x0F,
        }
    }
    
    pub fn set_light(&mut self, x: u32, y: u32, z: u32, light: LightLevel) {
        if x >= self.size || y >= self.size || z >= self.size {
            return;
        }
        
        let idx = self.index(x, y, z);
        let packed = ((light.sky & 0x0F) << 4) | (light.block & 0x0F);
        self.data[idx] = packed;
    }
    
    pub fn set_sky_light(&mut self, x: u32, y: u32, z: u32, level: u8) {
        if x >= self.size || y >= self.size || z >= self.size {
            return;
        }
        
        let idx = self.index(x, y, z);
        let block_light = self.data[idx] & 0x0F;
        self.data[idx] = ((level.min(15) & 0x0F) << 4) | block_light;
    }
    
    pub fn set_block_light(&mut self, x: u32, y: u32, z: u32, level: u8) {
        if x >= self.size || y >= self.size || z >= self.size {
            return;
        }
        
        let idx = self.index(x, y, z);
        let sky_light = self.data[idx] & 0xF0;
        self.data[idx] = sky_light | (level.min(15) & 0x0F);
    }
    
    /// Clear all light data
    pub fn clear(&mut self) {
        self.data.fill(0);
    }
    
    /// Fill with full skylight (for initialization)
    pub fn fill_sky(&mut self) {
        self.data.fill(0xF0); // Full sky light, no block light
    }
}