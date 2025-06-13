/// Data-Oriented Axis-Aligned Bounding Box System
/// 
/// Sprint 37: Converted from OOP to pure functions operating on AABB data.
/// Pure functions for collision detection - no methods, just data transformations.

use cgmath::{Vector3, Point3};

/// Axis-Aligned Bounding Box - pure data structure
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Point3<f32>,
    pub max: Point3<f32>,
}

/// Pure functions for AABB operations
/// No methods - just data transformations following DOP principles

/// Create new AABB from min/max points
/// Pure function - constructs AABB data structure
pub fn create_aabb(min: Point3<f32>, max: Point3<f32>) -> AABB {
    AABB { min, max }
}

/// Create AABB from center point and half extents
/// Pure function - transforms center/extents into AABB bounds
pub fn aabb_from_center_half_extents(center: Point3<f32>, half_extents: Vector3<f32>) -> AABB {
    AABB {
        min: Point3::new(
            center.x - half_extents.x,
            center.y - half_extents.y,
            center.z - half_extents.z,
        ),
        max: Point3::new(
            center.x + half_extents.x,
            center.y + half_extents.y,
            center.z + half_extents.z,
        ),
    }
}

/// Get center point of AABB
/// Pure function - calculates center from min/max bounds
pub fn aabb_center(aabb: &AABB) -> Point3<f32> {
    Point3::new(
        (aabb.min.x + aabb.max.x) * 0.5,
        (aabb.min.y + aabb.max.y) * 0.5,
        (aabb.min.z + aabb.max.z) * 0.5,
    )
}

/// Get half extents of AABB
/// Pure function - calculates half extents from bounds
pub fn aabb_half_extents(aabb: &AABB) -> Vector3<f32> {
    Vector3::new(
        (aabb.max.x - aabb.min.x) * 0.5,
        (aabb.max.y - aabb.min.y) * 0.5,
        (aabb.max.z - aabb.min.z) * 0.5,
    )
}

/// Test if two AABBs intersect
/// Pure function - tests intersection between two AABB data structures
pub fn aabb_intersects(aabb1: &AABB, aabb2: &AABB) -> bool {
    aabb1.min.x <= aabb2.max.x && aabb1.max.x >= aabb2.min.x &&
    aabb1.min.y <= aabb2.max.y && aabb1.max.y >= aabb2.min.y &&
    aabb1.min.z <= aabb2.max.z && aabb1.max.z >= aabb2.min.z
}

/// Test if AABB contains a point
/// Pure function - tests point containment
pub fn aabb_contains_point(aabb: &AABB, point: Point3<f32>) -> bool {
    point.x >= aabb.min.x && point.x <= aabb.max.x &&
    point.y >= aabb.min.y && point.y <= aabb.max.y &&
    point.z >= aabb.min.z && point.z <= aabb.max.z
}

/// Translate AABB by offset (mutating)
/// Function - transforms AABB data by offset
pub fn aabb_translate(aabb: &mut AABB, offset: Vector3<f32>) {
    aabb.min += offset;
    aabb.max += offset;
}

/// Create translated copy of AABB
/// Pure function - creates new AABB translated by offset
pub fn aabb_translated(aabb: &AABB, offset: Vector3<f32>) -> AABB {
    AABB {
        min: aabb.min + offset,
        max: aabb.max + offset,
    }
}

/// Calculate penetration depth and direction for collision resolution
/// Pure function - calculates separation vector between intersecting AABBs
pub fn aabb_penetration_vector(aabb1: &AABB, aabb2: &AABB) -> Option<Vector3<f32>> {
    if !aabb_intersects(aabb1, aabb2) {
        return None;
    }
    
    let x_overlap = (aabb1.max.x.min(aabb2.max.x) - aabb1.min.x.max(aabb2.min.x)).abs();
    let y_overlap = (aabb1.max.y.min(aabb2.max.y) - aabb1.min.y.max(aabb2.min.y)).abs();
    let z_overlap = (aabb1.max.z.min(aabb2.max.z) - aabb1.min.z.max(aabb2.min.z)).abs();
    
    // Find the axis with minimum overlap
    if x_overlap <= y_overlap && x_overlap <= z_overlap {
        let sign = if aabb_center(aabb1).x < aabb_center(aabb2).x { -1.0 } else { 1.0 };
        Some(Vector3::new(x_overlap * sign, 0.0, 0.0))
    } else if y_overlap <= x_overlap && y_overlap <= z_overlap {
        let sign = if aabb_center(aabb1).y < aabb_center(aabb2).y { -1.0 } else { 1.0 };
        Some(Vector3::new(0.0, y_overlap * sign, 0.0))
    } else {
        let sign = if aabb_center(aabb1).z < aabb_center(aabb2).z { -1.0 } else { 1.0 };
        Some(Vector3::new(0.0, 0.0, z_overlap * sign))
    }
}

/// Swept AABB collision detection
/// Pure function - calculates time of impact for moving AABB
pub fn aabb_swept_collision(aabb: &AABB, velocity: Vector3<f32>, other: &AABB, dt: f32) -> Option<f32> {
    // Expand the other AABB by this AABB's size
    let half_extents = aabb_half_extents(aabb);
    let expanded = AABB {
        min: Point3::new(
            other.min.x - half_extents.x * 2.0,
            other.min.y - half_extents.y * 2.0,
            other.min.z - half_extents.z * 2.0,
        ),
        max: Point3::new(
            other.max.x + half_extents.x * 2.0,
            other.max.y + half_extents.y * 2.0,
            other.max.z + half_extents.z * 2.0,
        ),
    };
    
    // Ray cast from center against expanded AABB
    let ray_origin = aabb_center(aabb);
    let ray_dir = velocity * dt;
    
    // If velocity is zero, no collision
    if ray_dir.x == 0.0 && ray_dir.y == 0.0 && ray_dir.z == 0.0 {
        return None;
    }
    
    // Calculate t values for each axis
    let mut t_min: f32 = 0.0;
    let mut t_max: f32 = 1.0;
    
    for i in 0..3 {
        let origin = match i {
            0 => ray_origin.x,
            1 => ray_origin.y,
            _ => ray_origin.z,
        };
        let dir = match i {
            0 => ray_dir.x,
            1 => ray_dir.y,
            _ => ray_dir.z,
        };
        let box_min = match i {
            0 => expanded.min.x,
            1 => expanded.min.y,
            _ => expanded.min.z,
        };
        let box_max = match i {
            0 => expanded.max.x,
            1 => expanded.max.y,
            _ => expanded.max.z,
        };
        
        if dir.abs() < 1e-6 {
            // Ray is parallel to axis
            if origin < box_min || origin > box_max {
                return None;
            }
        } else {
            let t1 = (box_min - origin) / dir;
            let t2 = (box_max - origin) / dir;
            
            let t_near = t1.min(t2);
            let t_far = t1.max(t2);
            
            t_min = t_min.max(t_near);
            t_max = t_max.min(t_far);
            
            if t_min > t_max {
                return None;
            }
        }
    }
    
    if t_min >= 0.0 && t_min <= 1.0 {
        Some(t_min)
    } else {
        None
    }
}

// ===== COMPATIBILITY LAYER =====
// Temporary methods for code that hasn't been converted yet

impl AABB {
    /// Compatibility wrapper - use create_aabb function instead
    #[deprecated(note = "Use create_aabb function instead")]
    pub fn new(min: Point3<f32>, max: Point3<f32>) -> Self {
        create_aabb(min, max)
    }
    
    /// Compatibility wrapper - use aabb_from_center_half_extents function instead
    #[deprecated(note = "Use aabb_from_center_half_extents function instead")]
    pub fn from_center_half_extents(center: Point3<f32>, half_extents: Vector3<f32>) -> Self {
        aabb_from_center_half_extents(center, half_extents)
    }
    
    /// Compatibility wrapper - use aabb_center function instead
    #[deprecated(note = "Use aabb_center function instead")]
    pub fn center(&self) -> Point3<f32> {
        aabb_center(self)
    }
    
    /// Compatibility wrapper - use aabb_half_extents function instead
    #[deprecated(note = "Use aabb_half_extents function instead")]
    pub fn half_extents(&self) -> Vector3<f32> {
        aabb_half_extents(self)
    }
    
    /// Compatibility wrapper - use aabb_intersects function instead
    #[deprecated(note = "Use aabb_intersects function instead")]
    pub fn intersects(&self, other: &AABB) -> bool {
        aabb_intersects(self, other)
    }
    
    /// Compatibility wrapper - use aabb_contains_point function instead
    #[deprecated(note = "Use aabb_contains_point function instead")]
    pub fn contains_point(&self, point: Point3<f32>) -> bool {
        aabb_contains_point(self, point)
    }
    
    /// Compatibility wrapper - use aabb_translate function instead
    #[deprecated(note = "Use aabb_translate function instead")]
    pub fn translate(&mut self, offset: Vector3<f32>) {
        aabb_translate(self, offset)
    }
    
    /// Compatibility wrapper - use aabb_translated function instead
    #[deprecated(note = "Use aabb_translated function instead")]
    pub fn translated(&self, offset: Vector3<f32>) -> Self {
        aabb_translated(self, offset)
    }
    
    /// Compatibility wrapper - use aabb_penetration_vector function instead
    #[deprecated(note = "Use aabb_penetration_vector function instead")]
    pub fn penetration_vector(&self, other: &AABB) -> Option<Vector3<f32>> {
        aabb_penetration_vector(self, other)
    }
    
    /// Compatibility wrapper - use aabb_swept_collision function instead
    #[deprecated(note = "Use aabb_swept_collision function instead")]
    pub fn swept_collision(&self, velocity: Vector3<f32>, other: &AABB, dt: f32) -> Option<f32> {
        aabb_swept_collision(self, velocity, other, dt)
    }
}