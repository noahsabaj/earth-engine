
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
#[derive(Clone)]
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
}

/// Calculate index for light map coordinates
/// Pure function - computes array index from 3D coordinates
fn light_map_index(light_map: &LightMap, x: u32, y: u32, z: u32) -> usize {
    (y * light_map.size * light_map.size + z * light_map.size + x) as usize
}

/// Get light level at coordinates
/// Pure function - reads light data from map
pub fn get_light_from_map(light_map: &LightMap, x: u32, y: u32, z: u32) -> LightLevel {
    if x >= light_map.size || y >= light_map.size || z >= light_map.size {
        return LightLevel::dark();
    }
    
    let packed = light_map.data[light_map_index(light_map, x, y, z)];
    LightLevel {
        sky: (packed >> 4) & 0x0F,
        block: packed & 0x0F,
    }
}

/// Set light level at coordinates
/// Function - transforms light map data
pub fn set_light_in_map(light_map: &mut LightMap, x: u32, y: u32, z: u32, light: LightLevel) {
    if x >= light_map.size || y >= light_map.size || z >= light_map.size {
        return;
    }
    
    let idx = light_map_index(light_map, x, y, z);
    let packed = ((light.sky & 0x0F) << 4) | (light.block & 0x0F);
    light_map.data[idx] = packed;
}

/// Set sky light level at coordinates
/// Function - transforms light map sky data
pub fn set_sky_light_in_map(light_map: &mut LightMap, x: u32, y: u32, z: u32, level: u8) {
    if x >= light_map.size || y >= light_map.size || z >= light_map.size {
        return;
    }
    
    let idx = light_map_index(light_map, x, y, z);
    let block_light = light_map.data[idx] & 0x0F;
    light_map.data[idx] = ((level.min(15) & 0x0F) << 4) | block_light;
}

/// Set block light level at coordinates
/// Function - transforms light map block data
pub fn set_block_light_in_map(light_map: &mut LightMap, x: u32, y: u32, z: u32, level: u8) {
    if x >= light_map.size || y >= light_map.size || z >= light_map.size {
        return;
    }
    
    let idx = light_map_index(light_map, x, y, z);
    let sky_light = light_map.data[idx] & 0xF0;
    light_map.data[idx] = sky_light | (level.min(15) & 0x0F);
}

/// Clear all light data
/// Function - transforms light map to empty state
pub fn clear_light_map(light_map: &mut LightMap) {
    light_map.data.fill(0);
}

/// Fill with full skylight (for initialization)
/// Function - transforms light map to full sky lighting
pub fn fill_sky_light_map(light_map: &mut LightMap) {
    light_map.data.fill(0xF0); // Full sky light, no block light
}