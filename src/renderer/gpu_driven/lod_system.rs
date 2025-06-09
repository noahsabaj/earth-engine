use cgmath::{Vector3, InnerSpace};

/// Level of detail configuration
#[derive(Debug, Clone, Copy)]
pub struct LodLevel {
    /// Minimum distance for this LOD
    pub min_distance: f32,
    
    /// Maximum distance for this LOD
    pub max_distance: f32,
    
    /// Index into mesh array for this LOD
    pub mesh_index: u32,
    
    /// Reduction factor (0.0 = no vertices, 1.0 = full detail)
    pub detail_factor: f32,
}

impl LodLevel {
    pub fn new(min_distance: f32, max_distance: f32, mesh_index: u32, detail_factor: f32) -> Self {
        Self {
            min_distance,
            max_distance,
            mesh_index,
            detail_factor,
        }
    }
}

/// LOD configuration for a mesh
#[derive(Debug, Clone)]
pub struct LodConfig {
    /// Available LOD levels
    pub levels: Vec<LodLevel>,
    
    /// Base mesh bounding radius
    pub bounding_radius: f32,
    
    /// Screen space error threshold
    pub error_threshold: f32,
}

impl LodConfig {
    pub fn new(bounding_radius: f32) -> Self {
        Self {
            levels: Vec::new(),
            bounding_radius,
            error_threshold: 5.0, // 5 pixels
        }
    }
    
    /// Add a LOD level
    pub fn add_level(&mut self, level: LodLevel) {
        self.levels.push(level);
        // Keep sorted by min distance
        self.levels.sort_by(|a, b| a.min_distance.partial_cmp(&b.min_distance).unwrap());
    }
    
    /// Select LOD based on distance
    pub fn select_lod_by_distance(&self, distance: f32) -> Option<&LodLevel> {
        for level in &self.levels {
            if distance >= level.min_distance && distance < level.max_distance {
                return Some(level);
            }
        }
        None
    }
    
    /// Select LOD based on screen space error
    pub fn select_lod_by_screen_size(
        &self,
        distance: f32,
        screen_height: f32,
        fov_y: f32,
    ) -> Option<&LodLevel> {
        // Calculate screen space size
        let screen_size = (self.bounding_radius * screen_height) / (distance * fov_y.tan());
        
        // Select LOD based on screen size
        // Higher detail for larger screen sizes
        let detail_needed = (screen_size / self.error_threshold).min(1.0);
        
        // Find best matching LOD
        let mut best_lod = None;
        let mut best_diff = f32::MAX;
        
        for level in &self.levels {
            let diff = (level.detail_factor - detail_needed).abs();
            if diff < best_diff && distance >= level.min_distance && distance < level.max_distance {
                best_diff = diff;
                best_lod = Some(level);
            }
        }
        
        best_lod
    }
}

/// Manages LOD configurations for different mesh types
pub struct LodSystem {
    /// LOD configurations by mesh type ID
    configs: std::collections::HashMap<u32, LodConfig>,
    
    /// Global LOD bias (negative = higher detail, positive = lower detail)
    lod_bias: f32,
    
    /// Maximum draw distance
    max_draw_distance: f32,
}

impl LodSystem {
    pub fn new() -> Self {
        Self {
            configs: std::collections::HashMap::new(),
            lod_bias: 0.0,
            max_draw_distance: 1000.0,
        }
    }
    
    /// Register LOD configuration for a mesh type
    pub fn register_config(&mut self, mesh_type_id: u32, config: LodConfig) {
        self.configs.insert(mesh_type_id, config);
    }
    
    /// Set global LOD bias
    pub fn set_lod_bias(&mut self, bias: f32) {
        self.lod_bias = bias.clamp(-2.0, 2.0);
    }
    
    /// Select LOD for an object
    pub fn select_lod(
        &self,
        mesh_type_id: u32,
        object_position: Vector3<f32>,
        camera_position: Vector3<f32>,
        screen_height: f32,
        fov_y: f32,
    ) -> Option<LodSelection> {
        let config = self.configs.get(&mesh_type_id)?;
        
        // Calculate distance
        let distance = (object_position - camera_position).magnitude();
        
        // Apply LOD bias to distance
        let biased_distance = distance * (1.0 + self.lod_bias * 0.5);
        
        // Check max draw distance
        if biased_distance > self.max_draw_distance {
            return None; // Don't draw
        }
        
        // Select LOD
        let lod = if screen_height > 0.0 && fov_y > 0.0 {
            config.select_lod_by_screen_size(biased_distance, screen_height, fov_y)?
        } else {
            config.select_lod_by_distance(biased_distance)?
        };
        
        Some(LodSelection {
            level: lod.mesh_index,
            distance,
            detail_factor: lod.detail_factor,
        })
    }
    
    /// Create default LOD config for chunks
    pub fn create_chunk_lod_config() -> LodConfig {
        let mut config = LodConfig::new(32.0); // Chunk radius
        
        // LOD 0: Full detail (0-50m)
        config.add_level(LodLevel::new(0.0, 50.0, 0, 1.0));
        
        // LOD 1: Half detail (50-150m)
        config.add_level(LodLevel::new(50.0, 150.0, 1, 0.5));
        
        // LOD 2: Quarter detail (150-400m)
        config.add_level(LodLevel::new(150.0, 400.0, 2, 0.25));
        
        // LOD 3: Minimal detail (400-1000m)
        config.add_level(LodLevel::new(400.0, 1000.0, 3, 0.1));
        
        config
    }
    
    /// Create default LOD config for entities
    pub fn create_entity_lod_config(radius: f32) -> LodConfig {
        let mut config = LodConfig::new(radius);
        
        // Scale distances based on entity size
        let scale = radius.max(1.0);
        
        // LOD 0: Full detail
        config.add_level(LodLevel::new(0.0, 20.0 * scale, 0, 1.0));
        
        // LOD 1: Medium detail
        config.add_level(LodLevel::new(20.0 * scale, 50.0 * scale, 1, 0.6));
        
        // LOD 2: Low detail
        config.add_level(LodLevel::new(50.0 * scale, 100.0 * scale, 2, 0.3));
        
        config
    }
}

/// Result of LOD selection
#[derive(Debug, Clone, Copy)]
pub struct LodSelection {
    /// Selected LOD level index
    pub level: u32,
    
    /// Distance from camera
    pub distance: f32,
    
    /// Detail factor for this LOD
    pub detail_factor: f32,
}

/// LOD transition modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LodTransition {
    /// Instant switch between LODs
    Instant,
    
    /// Fade between LODs (alpha blending)
    Fade { duration: f32 },
    
    /// Dither between LODs
    Dither { pattern_size: u32 },
}

impl Default for LodTransition {
    fn default() -> Self {
        Self::Fade { duration: 0.5 }
    }
}