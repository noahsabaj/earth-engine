#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_safe_spawn_height() {
        // Create a test world generator
        let generator = DefaultWorldGenerator::new(
            12345,
            BlockId(1), // grass
            BlockId(2), // dirt
            BlockId(3), // stone
            BlockId(4), // water
            BlockId(5), // sand
        );
        
        // Test spawn height at origin
        let spawn_height = generator.find_safe_spawn_height(0.0, 0.0);
        
        // Verify spawn height is reasonable
        assert!(spawn_height >= 20.0, "Spawn height too low: {}", spawn_height);
        assert!(spawn_height <= 250.0, "Spawn height too high: {}", spawn_height);
        
        // Test that spawn height is above terrain
        let terrain_height = generator.get_surface_height(0.0, 0.0);
        assert!(spawn_height > terrain_height as f32, 
                "Spawn height {} should be above terrain height {}", 
                spawn_height, terrain_height);
        
        // Should have at least 2-3 blocks clearance
        assert!(spawn_height >= terrain_height as f32 + 2.0,
                "Not enough clearance above terrain");
    }
    
    #[test]
    fn test_spawn_height_various_locations() {
        let generator = DefaultWorldGenerator::new(
            12345,
            BlockId(1), // grass
            BlockId(2), // dirt
            BlockId(3), // stone
            BlockId(4), // water
            BlockId(5), // sand
        );
        
        // Test various locations
        let test_positions = [
            (0.0, 0.0),
            (100.0, 100.0),
            (-100.0, -100.0),
            (1000.0, 0.0),
            (0.0, 1000.0),
        ];
        
        for (x, z) in test_positions {
            let spawn_height = generator.find_safe_spawn_height(x, z);
            let terrain_height = generator.get_surface_height(x, z);
            
            // Verify spawn is above terrain with clearance
            assert!(spawn_height > terrain_height as f32 + 2.0,
                    "At ({}, {}): spawn {} not high enough above terrain {}",
                    x, z, spawn_height, terrain_height);
            
            // Verify within bounds
            assert!(spawn_height >= 20.0 && spawn_height <= 250.0,
                    "At ({}, {}): spawn height {} out of bounds",
                    x, z, spawn_height);
        }
    }
}