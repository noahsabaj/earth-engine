#![allow(unused_variables, dead_code)]
use crate::sdf::SdfBuffer;
use crate::world_gpu::WorldBuffer;
use crate::physics::AABB;
use glam::Vec3;
use cgmath::Point3;
use std::sync::Arc;
use wgpu::Device;

/// Collision detection mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CollisionMode {
    /// Use voxel-based collision (precise, blocky)
    Voxel,
    
    /// Use SDF-based collision (smooth)
    Sdf,
    
    /// Hybrid mode based on context
    Hybrid,
}

/// Ray hit information
#[derive(Debug, Clone)]
pub struct HybridRayHit {
    /// Hit position
    pub position: Vec3,
    
    /// Surface normal
    pub normal: Vec3,
    
    /// Distance along ray
    pub distance: f32,
    
    /// Material at hit point
    pub material: u16,
    
    /// Whether hit was on voxel or SDF
    pub hit_type: CollisionMode,
}

/// Hybrid collision detector
pub struct HybridCollider {
    /// Current collision mode
    mode: CollisionMode,
    
    /// SDF gradient threshold for edge detection
    gradient_threshold: f32,
    
    /// Distance threshold for mode switching
    switch_distance: f32,
    
    /// Device reference
    device: Arc<Device>,
}

impl HybridCollider {
    /// Create new hybrid collider
    pub fn new(device: Arc<Device>) -> Self {
        Self {
            mode: CollisionMode::Hybrid,
            gradient_threshold: 0.5,
            switch_distance: 10.0,
            device,
        }
    }
    
    /// Set collision mode
    pub fn set_mode(&mut self, mode: CollisionMode) {
        self.mode = mode;
    }
    
    /// Perform sphere collision test
    pub fn collide_sphere(
        &self,
        center: Vec3,
        radius: f32,
        world_buffer: &WorldBuffer,
        sdf_buffer: Option<&SdfBuffer>,
    ) -> Option<Vec3> {
        match self.mode {
            CollisionMode::Voxel => self.collide_sphere_voxel(center, radius, world_buffer),
            CollisionMode::Sdf => {
                if let Some(sdf) = sdf_buffer {
                    self.collide_sphere_sdf(center, radius, sdf)
                } else {
                    self.collide_sphere_voxel(center, radius, world_buffer)
                }
            }
            CollisionMode::Hybrid => {
                // Use SDF for smooth collision if available and close enough
                if let Some(sdf) = sdf_buffer {
                    if self.should_use_sdf(center) {
                        self.collide_sphere_sdf(center, radius, sdf)
                    } else {
                        self.collide_sphere_voxel(center, radius, world_buffer)
                    }
                } else {
                    self.collide_sphere_voxel(center, radius, world_buffer)
                }
            }
        }
    }
    
    /// Voxel-based sphere collision
    fn collide_sphere_voxel(
        &self,
        center: Vec3,
        radius: f32,
        world_buffer: &WorldBuffer,
    ) -> Option<Vec3> {
        // Check voxels in sphere bounds
        let min = center - Vec3::splat(radius);
        let max = center + Vec3::splat(radius);
        
        let min_voxel = min.floor().as_ivec3();
        let max_voxel = max.ceil().as_ivec3();
        
        let mut penetration = Vec3::ZERO;
        let mut hit = false;
        
        // Simple voxel collision - check each voxel in bounds
        for z in min_voxel.z..=max_voxel.z {
            for y in min_voxel.y..=max_voxel.y {
                for x in min_voxel.x..=max_voxel.x {
                    let voxel_center = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
                    let voxel_aabb = AABB {
                        min: Point3::new(x as f32, y as f32, z as f32),
                        max: Point3::new((x + 1) as f32, (y + 1) as f32, (z + 1) as f32),
                    };
                    
                    // Check if voxel exists (simplified - would query world_buffer)
                    // For now, assume collision detection logic
                    
                    // Calculate sphere-AABB penetration
                    let closest = voxel_aabb.closest_point(center);
                    let distance = (center - closest).length();
                    
                    if distance < radius {
                        let normal = (center - closest).normalize();
                        let depth = radius - distance;
                        penetration += normal * depth;
                        hit = true;
                    }
                }
            }
        }
        
        if hit {
            Some(penetration)
        } else {
            None
        }
    }
    
    /// SDF-based sphere collision
    fn collide_sphere_sdf(
        &self,
        center: Vec3,
        radius: f32,
        sdf_buffer: &SdfBuffer,
    ) -> Option<Vec3> {
        // Sample SDF at sphere center
        let sdf_distance = self.sample_sdf(center, sdf_buffer);
        
        if sdf_distance < radius {
            // Calculate gradient for normal
            let gradient = self.calculate_sdf_gradient(center, sdf_buffer);
            let normal = gradient.normalize();
            
            // Penetration depth
            let depth = radius - sdf_distance;
            
            Some(normal * depth)
        } else {
            None
        }
    }
    
    /// Cast ray through hybrid collision system
    pub fn cast_ray(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        world_buffer: &WorldBuffer,
        sdf_buffer: Option<&SdfBuffer>,
    ) -> Option<HybridRayHit> {
        let direction = direction.normalize();
        
        match self.mode {
            CollisionMode::Voxel => self.cast_ray_voxel(origin, direction, max_distance, world_buffer),
            CollisionMode::Sdf => {
                if let Some(sdf) = sdf_buffer {
                    self.cast_ray_sdf(origin, direction, max_distance, sdf)
                } else {
                    self.cast_ray_voxel(origin, direction, max_distance, world_buffer)
                }
            }
            CollisionMode::Hybrid => {
                // Try SDF first for smooth results
                if let Some(sdf) = sdf_buffer {
                    if let Some(hit) = self.cast_ray_sdf(origin, direction, max_distance, sdf) {
                        return Some(hit);
                    }
                }
                // Fall back to voxel
                self.cast_ray_voxel(origin, direction, max_distance, world_buffer)
            }
        }
    }
    
    /// Voxel-based ray casting (DDA algorithm)
    fn cast_ray_voxel(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        world_buffer: &WorldBuffer,
    ) -> Option<HybridRayHit> {
        // Simplified DDA implementation
        let mut current_pos = origin;
        let step = direction * 0.1; // Small steps for now
        let mut distance = 0.0;
        
        while distance < max_distance {
            let voxel_pos = current_pos.floor().as_ivec3();
            
            // Check if voxel is solid (simplified)
            // In practice, would query world_buffer
            
            current_pos += step;
            distance += 0.1;
        }
        
        None // Simplified - would return actual hit
    }
    
    /// SDF-based ray marching
    fn cast_ray_sdf(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
        sdf_buffer: &SdfBuffer,
    ) -> Option<HybridRayHit> {
        let mut position = origin;
        let mut total_distance = 0.0;
        const EPSILON: f32 = 0.001;
        const MAX_STEPS: u32 = 128;
        
        for _ in 0..MAX_STEPS {
            let distance = self.sample_sdf(position, sdf_buffer);
            
            if distance < EPSILON {
                // Hit surface
                let normal = self.calculate_sdf_gradient(position, sdf_buffer).normalize();
                let material = self.sample_sdf_material(position, sdf_buffer);
                
                return Some(HybridRayHit {
                    position,
                    normal,
                    distance: total_distance,
                    material,
                    hit_type: CollisionMode::Sdf,
                });
            }
            
            // March forward
            position += direction * distance;
            total_distance += distance;
            
            if total_distance > max_distance {
                break;
            }
        }
        
        None
    }
    
    /// Sample SDF at world position
    fn sample_sdf(&self, position: Vec3, sdf_buffer: &SdfBuffer) -> f32 {
        // Convert world position to SDF grid coordinates
        let sdf_pos = self.world_to_sdf_coords(position, sdf_buffer);
        
        // Trilinear interpolation
        // Simplified - would actually sample from GPU buffer
        8.0 // Placeholder
    }
    
    /// Sample material from SDF
    fn sample_sdf_material(&self, position: Vec3, sdf_buffer: &SdfBuffer) -> u16 {
        // Similar to sample_sdf but returns material ID
        1 // Placeholder
    }
    
    /// Calculate SDF gradient using finite differences
    fn calculate_sdf_gradient(&self, position: Vec3, sdf_buffer: &SdfBuffer) -> Vec3 {
        const H: f32 = 0.01;
        
        let dx = self.sample_sdf(position + Vec3::X * H, sdf_buffer) - 
                 self.sample_sdf(position - Vec3::X * H, sdf_buffer);
        let dy = self.sample_sdf(position + Vec3::Y * H, sdf_buffer) - 
                 self.sample_sdf(position - Vec3::Y * H, sdf_buffer);
        let dz = self.sample_sdf(position + Vec3::Z * H, sdf_buffer) - 
                 self.sample_sdf(position - Vec3::Z * H, sdf_buffer);
        
        Vec3::new(dx, dy, dz) / (2.0 * H)
    }
    
    /// Convert world coordinates to SDF grid coordinates
    fn world_to_sdf_coords(&self, world_pos: Vec3, sdf_buffer: &SdfBuffer) -> Vec3 {
        let offset = Vec3::new(
            sdf_buffer.world_offset.0 as f32,
            sdf_buffer.world_offset.1 as f32,
            sdf_buffer.world_offset.2 as f32,
        );
        
        (world_pos - offset) * super::SDF_RESOLUTION_FACTOR
    }
    
    /// Determine if SDF should be used based on context
    fn should_use_sdf(&self, position: Vec3) -> bool {
        // Use SDF for distant/smooth areas
        // Use voxels for precise/close interactions
        true // Simplified
    }
    
    /// Get penetration depth at position
    pub fn get_penetration_depth(
        &self,
        position: Vec3,
        sdf_buffer: &SdfBuffer,
    ) -> f32 {
        -self.sample_sdf(position, sdf_buffer)
    }
}

/// AABB extension for closest point calculation
impl AABB {
    /// Find closest point on AABB to given point
    pub fn closest_point(&self, point: Vec3) -> Vec3 {
        Vec3::new(
            point.x.clamp(self.min.x, self.max.x),
            point.y.clamp(self.min.y, self.max.y),
            point.z.clamp(self.min.z, self.max.z),
        )
    }
}

/// Collision response data
pub struct CollisionResponse {
    /// Separation vector
    pub separation: Vec3,
    
    /// Contact points
    pub contacts: Vec<ContactPoint>,
    
    /// Total penetration depth
    pub depth: f32,
}

/// Contact point information
pub struct ContactPoint {
    /// World position
    pub position: Vec3,
    
    /// Contact normal
    pub normal: Vec3,
    
    /// Penetration depth
    pub depth: f32,
    
    /// Material at contact
    pub material: u16,
}