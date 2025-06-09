use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use crate::biome::{BiomeType, BiomeProperties};
use crate::world::{Chunk, ChunkPos, BlockId};

/// Features that can be placed in biomes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationFeature {
    Tree,
    Flower,
    Grass,
    Cactus,
    DeadBush,
    Mushroom,
    SugarCane,
    Pumpkin,
    Ore(OreType),
}

/// Types of ores that can generate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OreType {
    Coal,
    Iron,
    Gold,
    Diamond,
    Redstone,
    Lapis,
    Emerald,
}

/// Decorates chunks with biome-specific features
pub struct BiomeDecorator {
    seed: u64,
}

impl BiomeDecorator {
    /// Create a new biome decorator
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }
    
    /// Decorate a chunk based on its biome
    pub fn decorate_chunk(
        &self,
        chunk: &mut Chunk,
        chunk_pos: ChunkPos,
        chunk_size: u32,
        biome: BiomeType,
    ) {
        let props = BiomeProperties::from_biome_type(biome);
        let mut rng = self.create_rng(chunk_pos);
        
        // Generate ores first (underground)
        self.generate_ores(chunk, chunk_pos, chunk_size, &mut rng);
        
        // Surface decorations
        self.generate_trees(chunk, chunk_pos, chunk_size, &props, &mut rng);
        self.generate_grass(chunk, chunk_pos, chunk_size, &props, &mut rng);
        self.generate_flowers(chunk, chunk_pos, chunk_size, &props, &mut rng);
        
        // Biome-specific decorations
        match biome {
            BiomeType::Desert | BiomeType::Badlands => {
                self.generate_cacti(chunk, chunk_pos, chunk_size, &mut rng);
                self.generate_dead_bushes(chunk, chunk_pos, chunk_size, &mut rng);
            },
            BiomeType::Swamp => {
                self.generate_mushrooms(chunk, chunk_pos, chunk_size, &mut rng);
                self.generate_sugar_cane(chunk, chunk_pos, chunk_size, &mut rng);
            },
            BiomeType::Jungle => {
                self.generate_vines(chunk, chunk_pos, chunk_size, &mut rng);
            },
            _ => {},
        }
    }
    
    /// Create RNG for consistent decoration
    fn create_rng(&self, chunk_pos: ChunkPos) -> StdRng {
        let chunk_seed = self.seed
            .wrapping_add(chunk_pos.x as u64)
            .wrapping_mul(73856093)
            .wrapping_add(chunk_pos.z as u64)
            .wrapping_mul(19349663);
        StdRng::seed_from_u64(chunk_seed)
    }
    
    /// Generate trees
    fn generate_trees(
        &self,
        chunk: &mut Chunk,
        _chunk_pos: ChunkPos,
        chunk_size: u32,
        props: &BiomeProperties,
        rng: &mut StdRng,
    ) {
        let tree_count = (props.tree_density * chunk_size as f32 * chunk_size as f32 / 256.0) as u32;
        
        for _ in 0..tree_count {
            let x = rng.gen_range(2..chunk_size - 2);
            let z = rng.gen_range(2..chunk_size - 2);
            
            // Find surface
            if let Some(y) = self.find_surface(chunk, x, z, chunk_size) {
                // Check if we can place a tree
                if chunk.get_block(x, y, z) == props.surface_block {
                    self.place_tree(chunk, x, y + 1, z, rng);
                }
            }
        }
    }
    
    /// Place a tree at position
    fn place_tree(&self, chunk: &mut Chunk, x: u32, y: u32, z: u32, rng: &mut StdRng) {
        let height = rng.gen_range(4..7);
        
        // Trunk
        for h in 0..height {
            if y + h < chunk.size() {
                chunk.set_block(x, y + h, z, BlockId::LOG);
            }
        }
        
        // Leaves (simple sphere)
        let leaf_start = height - 2;
        let leaf_radius = 2;
        
        for dy in 0..4 {
            let leaf_y = y + leaf_start + dy;
            if leaf_y >= chunk.size() {
                continue;
            }
            
            let radius = if dy < 2 { leaf_radius } else { leaf_radius - 1 };
            
            for dx in -(radius as i32)..=(radius as i32) {
                for dz in -(radius as i32)..=(radius as i32) {
                    let nx = x as i32 + dx;
                    let nz = z as i32 + dz;
                    
                    if nx >= 0 && nx < chunk.size() as i32 && nz >= 0 && nz < chunk.size() as i32 {
                        let dist_sq = dx * dx + dz * dz;
                        if dist_sq <= radius * radius as i32 {
                            if chunk.get_block(nx as u32, leaf_y, nz as u32) == BlockId::AIR {
                                chunk.set_block(nx as u32, leaf_y, nz as u32, BlockId::LEAVES);
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Generate grass blocks
    fn generate_grass(
        &self,
        chunk: &mut Chunk,
        _chunk_pos: ChunkPos,
        chunk_size: u32,
        props: &BiomeProperties,
        rng: &mut StdRng,
    ) {
        let grass_count = (props.grass_density * chunk_size as f32 * chunk_size as f32 / 16.0) as u32;
        
        for _ in 0..grass_count {
            let x = rng.gen_range(0..chunk_size);
            let z = rng.gen_range(0..chunk_size);
            
            if let Some(y) = self.find_surface(chunk, x, z, chunk_size) {
                if chunk.get_block(x, y, z) == props.surface_block && y + 1 < chunk_size {
                    chunk.set_block(x, y + 1, z, BlockId::TALL_GRASS);
                }
            }
        }
    }
    
    /// Generate flowers
    fn generate_flowers(
        &self,
        chunk: &mut Chunk,
        _chunk_pos: ChunkPos,
        chunk_size: u32,
        props: &BiomeProperties,
        rng: &mut StdRng,
    ) {
        let flower_count = (props.flower_density * chunk_size as f32 * chunk_size as f32 / 64.0) as u32;
        
        for _ in 0..flower_count {
            let x = rng.gen_range(0..chunk_size);
            let z = rng.gen_range(0..chunk_size);
            
            if let Some(y) = self.find_surface(chunk, x, z, chunk_size) {
                if chunk.get_block(x, y, z) == props.surface_block && y + 1 < chunk_size {
                    let flower_type = if rng.gen_bool(0.5) {
                        BlockId::FLOWER_RED
                    } else {
                        BlockId::FLOWER_YELLOW
                    };
                    chunk.set_block(x, y + 1, z, flower_type);
                }
            }
        }
    }
    
    /// Generate cacti (for desert biomes)
    fn generate_cacti(&self, chunk: &mut Chunk, _chunk_pos: ChunkPos, chunk_size: u32, rng: &mut StdRng) {
        let cactus_count = rng.gen_range(0..3);
        
        for _ in 0..cactus_count {
            let x = rng.gen_range(1..chunk_size - 1);
            let z = rng.gen_range(1..chunk_size - 1);
            
            if let Some(y) = self.find_surface(chunk, x, z, chunk_size) {
                if chunk.get_block(x, y, z) == BlockId::SAND {
                    let height = rng.gen_range(1..4);
                    for h in 0..height {
                        if y + h + 1 < chunk_size {
                            chunk.set_block(x, y + h + 1, z, BlockId::CACTUS);
                        }
                    }
                }
            }
        }
    }
    
    /// Generate dead bushes (for desert biomes)
    fn generate_dead_bushes(&self, chunk: &mut Chunk, _chunk_pos: ChunkPos, chunk_size: u32, rng: &mut StdRng) {
        let bush_count = rng.gen_range(0..4);
        
        for _ in 0..bush_count {
            let x = rng.gen_range(0..chunk_size);
            let z = rng.gen_range(0..chunk_size);
            
            if let Some(y) = self.find_surface(chunk, x, z, chunk_size) {
                if chunk.get_block(x, y, z) == BlockId::SAND && y + 1 < chunk_size {
                    chunk.set_block(x, y + 1, z, BlockId::DEAD_BUSH);
                }
            }
        }
    }
    
    /// Generate mushrooms (for dark/swamp biomes)
    fn generate_mushrooms(&self, chunk: &mut Chunk, _chunk_pos: ChunkPos, chunk_size: u32, rng: &mut StdRng) {
        let mushroom_count = rng.gen_range(0..5);
        
        for _ in 0..mushroom_count {
            let x = rng.gen_range(0..chunk_size);
            let z = rng.gen_range(0..chunk_size);
            
            if let Some(y) = self.find_surface(chunk, x, z, chunk_size) {
                if y + 1 < chunk_size {
                    let mushroom_type = if rng.gen_bool(0.5) {
                        BlockId::MUSHROOM_RED
                    } else {
                        BlockId::MUSHROOM_BROWN
                    };
                    chunk.set_block(x, y + 1, z, mushroom_type);
                }
            }
        }
    }
    
    /// Generate sugar cane (near water)
    fn generate_sugar_cane(&self, chunk: &mut Chunk, _chunk_pos: ChunkPos, chunk_size: u32, rng: &mut StdRng) {
        for _ in 0..3 {
            let x = rng.gen_range(1..chunk_size - 1);
            let z = rng.gen_range(1..chunk_size - 1);
            
            if let Some(y) = self.find_surface(chunk, x, z, chunk_size) {
                // Check if adjacent to water
                let mut near_water = false;
                for dx in -1..=1 {
                    for dz in -1..=1 {
                        if dx == 0 && dz == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let nz = z as i32 + dz;
                        if nx >= 0 && nx < chunk_size as i32 && nz >= 0 && nz < chunk_size as i32 {
                            if chunk.get_block(nx as u32, y, nz as u32) == BlockId::WATER {
                                near_water = true;
                                break;
                            }
                        }
                    }
                }
                
                if near_water && chunk.get_block(x, y, z) != BlockId::WATER {
                    let height = rng.gen_range(1..4);
                    for h in 0..height {
                        if y + h + 1 < chunk_size {
                            chunk.set_block(x, y + h + 1, z, BlockId::SUGAR_CANE);
                        }
                    }
                }
            }
        }
    }
    
    /// Generate vines (for jungle biomes)
    fn generate_vines(&self, chunk: &mut Chunk, _chunk_pos: ChunkPos, chunk_size: u32, rng: &mut StdRng) {
        // Find trees and add vines
        for x in 0..chunk_size {
            for z in 0..chunk_size {
                for y in 0..chunk_size {
                    if chunk.get_block(x, y, z) == BlockId::LOG {
                        // Chance to add vines on each side
                        for (dx, dz) in &[(1, 0), (-1, 0), (0, 1), (0, -1)] {
                            if rng.gen_bool(0.3) {
                                let nx = x as i32 + dx;
                                let nz = z as i32 + dz;
                                if nx >= 0 && nx < chunk_size as i32 && nz >= 0 && nz < chunk_size as i32 {
                                    if chunk.get_block(nx as u32, y, nz as u32) == BlockId::AIR {
                                        chunk.set_block(nx as u32, y, nz as u32, BlockId::VINES);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Generate ore veins
    fn generate_ores(&self, chunk: &mut Chunk, chunk_pos: ChunkPos, chunk_size: u32, rng: &mut StdRng) {
        // Coal - common, high up
        self.generate_ore_type(chunk, chunk_pos, chunk_size, OreType::Coal, 20, 16, 0, 128, rng);
        
        // Iron - common, medium depth
        self.generate_ore_type(chunk, chunk_pos, chunk_size, OreType::Iron, 20, 8, 0, 64, rng);
        
        // Gold - rare, deep
        self.generate_ore_type(chunk, chunk_pos, chunk_size, OreType::Gold, 2, 8, 0, 32, rng);
        
        // Diamond - very rare, very deep
        self.generate_ore_type(chunk, chunk_pos, chunk_size, OreType::Diamond, 1, 7, 0, 16, rng);
    }
    
    /// Generate a specific ore type
    fn generate_ore_type(
        &self,
        chunk: &mut Chunk,
        chunk_pos: ChunkPos,
        chunk_size: u32,
        ore_type: OreType,
        veins_per_chunk: u32,
        vein_size: u32,
        min_y: i32,
        max_y: i32,
        rng: &mut StdRng,
    ) {
        let ore_block = match ore_type {
            OreType::Coal => BlockId::COAL_ORE,
            OreType::Iron => BlockId::IRON_ORE,
            OreType::Gold => BlockId::GOLD_ORE,
            OreType::Diamond => BlockId::DIAMOND_ORE,
            _ => return, // Not implemented
        };
        
        for _ in 0..veins_per_chunk {
            let x = rng.gen_range(0..chunk_size);
            let z = rng.gen_range(0..chunk_size);
            let world_y = rng.gen_range(min_y..max_y);
            let y = world_y - chunk_pos.y * chunk_size as i32;
            
            if y >= 0 && y < chunk_size as i32 {
                self.place_ore_vein(chunk, x, y as u32, z, ore_block, vein_size, chunk_size, rng);
            }
        }
    }
    
    /// Place an ore vein
    fn place_ore_vein(
        &self,
        chunk: &mut Chunk,
        x: u32,
        y: u32,
        z: u32,
        ore_block: BlockId,
        size: u32,
        chunk_size: u32,
        rng: &mut StdRng,
    ) {
        let mut placed = 0;
        let mut positions = vec![(x, y, z)];
        
        while placed < size && !positions.is_empty() {
            let idx = rng.gen_range(0..positions.len());
            let (cx, cy, cz) = positions.remove(idx);
            
            // Place ore if it's stone
            if chunk.get_block(cx, cy, cz) == BlockId::STONE {
                chunk.set_block(cx, cy, cz, ore_block);
                placed += 1;
                
                // Add adjacent positions
                for (dx, dy, dz) in &[
                    (1, 0, 0), (-1, 0, 0),
                    (0, 1, 0), (0, -1, 0),
                    (0, 0, 1), (0, 0, -1),
                ] {
                    let nx = cx as i32 + dx;
                    let ny = cy as i32 + dy;
                    let nz = cz as i32 + dz;
                    
                    if nx >= 0 && nx < chunk_size as i32
                        && ny >= 0 && ny < chunk_size as i32
                        && nz >= 0 && nz < chunk_size as i32
                    {
                        positions.push((nx as u32, ny as u32, nz as u32));
                    }
                }
            }
        }
    }
    
    /// Find the surface at a position
    fn find_surface(&self, chunk: &Chunk, x: u32, z: u32, chunk_size: u32) -> Option<u32> {
        for y in (0..chunk_size).rev() {
            let block = chunk.get_block(x, y, z);
            if block != BlockId::AIR && block != BlockId::WATER {
                return Some(y);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_biome_decorator() {
        let decorator = BiomeDecorator::new(12345);
        let mut chunk = Chunk::new(ChunkPos { x: 0, y: 0, z: 0 }, 16);
        
        // Fill chunk with basic terrain
        for x in 0..16 {
            for z in 0..16 {
                for y in 0..10 {
                    chunk.set_block(x, y, z, BlockId::STONE);
                }
                chunk.set_block(x, 10, z, BlockId::GRASS);
                for y in 11..16 {
                    chunk.set_block(x, y, z, BlockId::AIR);
                }
            }
        }
        
        // Decorate
        decorator.decorate_chunk(&mut chunk, ChunkPos { x: 0, y: 0, z: 0 }, 16, BiomeType::Forest);
        
        // Should have some decorations
        let mut has_decorations = false;
        for x in 0..16 {
            for y in 11..16 {
                for z in 0..16 {
                    let block = chunk.get_block(x, y, z);
                    if block == BlockId::LOG || block == BlockId::LEAVES || block == BlockId::TALL_GRASS {
                        has_decorations = true;
                        break;
                    }
                }
            }
        }
        assert!(has_decorations);
    }
}