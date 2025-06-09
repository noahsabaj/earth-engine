use cgmath::{Vector3, Point3};

#[derive(Debug, Clone, Copy)]
pub struct AABB {
    pub min: Point3<f32>,
    pub max: Point3<f32>,
}

impl AABB {
    pub fn new(min: Point3<f32>, max: Point3<f32>) -> Self {
        Self { min, max }
    }
    
    pub fn from_center_half_extents(center: Point3<f32>, half_extents: Vector3<f32>) -> Self {
        Self {
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
    
    pub fn center(&self) -> Point3<f32> {
        Point3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }
    
    pub fn half_extents(&self) -> Vector3<f32> {
        Vector3::new(
            (self.max.x - self.min.x) * 0.5,
            (self.max.y - self.min.y) * 0.5,
            (self.max.z - self.min.z) * 0.5,
        )
    }
    
    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x &&
        self.min.y <= other.max.y && self.max.y >= other.min.y &&
        self.min.z <= other.max.z && self.max.z >= other.min.z
    }
    
    pub fn contains_point(&self, point: Point3<f32>) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }
    
    pub fn translate(&mut self, offset: Vector3<f32>) {
        self.min += offset;
        self.max += offset;
    }
    
    pub fn translated(&self, offset: Vector3<f32>) -> Self {
        Self {
            min: self.min + offset,
            max: self.max + offset,
        }
    }
    
    // Calculate penetration depth and direction for collision resolution
    pub fn penetration_vector(&self, other: &AABB) -> Option<Vector3<f32>> {
        if !self.intersects(other) {
            return None;
        }
        
        let x_overlap = (self.max.x.min(other.max.x) - self.min.x.max(other.min.x)).abs();
        let y_overlap = (self.max.y.min(other.max.y) - self.min.y.max(other.min.y)).abs();
        let z_overlap = (self.max.z.min(other.max.z) - self.min.z.max(other.min.z)).abs();
        
        // Find the axis with minimum overlap
        if x_overlap <= y_overlap && x_overlap <= z_overlap {
            let sign = if self.center().x < other.center().x { -1.0 } else { 1.0 };
            Some(Vector3::new(x_overlap * sign, 0.0, 0.0))
        } else if y_overlap <= x_overlap && y_overlap <= z_overlap {
            let sign = if self.center().y < other.center().y { -1.0 } else { 1.0 };
            Some(Vector3::new(0.0, y_overlap * sign, 0.0))
        } else {
            let sign = if self.center().z < other.center().z { -1.0 } else { 1.0 };
            Some(Vector3::new(0.0, 0.0, z_overlap * sign))
        }
    }
    
    // Swept AABB for continuous collision detection
    pub fn swept_collision(&self, velocity: Vector3<f32>, other: &AABB, dt: f32) -> Option<f32> {
        // Expand the other AABB by this AABB's size
        let expanded = AABB {
            min: Point3::new(
                other.min.x - self.half_extents().x * 2.0,
                other.min.y - self.half_extents().y * 2.0,
                other.min.z - self.half_extents().z * 2.0,
            ),
            max: Point3::new(
                other.max.x + self.half_extents().x * 2.0,
                other.max.y + self.half_extents().y * 2.0,
                other.max.z + self.half_extents().z * 2.0,
            ),
        };
        
        // Ray cast from center against expanded AABB
        let ray_origin = self.center();
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
}