use super::EntityType;
use serde::{Serialize, Deserialize};
use std::hash::{Hash, Hasher};

/// Result of a spatial query
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub entity_id: u64,
    pub distance: Option<f32>,
    pub entity_data: super::EntityData,
}

/// Type of spatial query
#[derive(Debug, Clone)]
pub enum QueryType {
    Range(RangeQuery),
    KNearest(KNearestQuery),
    Frustum(FrustumQuery),
    Box(BoxQuery),
}

/// A spatial query that can be cached
#[derive(Debug, Clone)]
pub struct SpatialQuery {
    query_type: QueryType,
    cache_key: u64,
}

impl SpatialQuery {
    pub fn range(center: [f32; 3], radius: f32) -> Self {
        let query = RangeQuery::new(center, radius);
        Self {
            cache_key: query.cache_key(),
            query_type: QueryType::Range(query),
        }
    }
    
    pub fn k_nearest(center: [f32; 3], k: usize) -> Self {
        let query = KNearestQuery::new(center, k);
        Self {
            cache_key: query.cache_key(),
            query_type: QueryType::KNearest(query),
        }
    }
    
    pub fn frustum(frustum: Frustum) -> Self {
        let query = FrustumQuery::new(frustum);
        Self {
            cache_key: query.cache_key(),
            query_type: QueryType::Frustum(query),
        }
    }
    
    pub fn box_query(min: [f32; 3], max: [f32; 3]) -> Self {
        let query = BoxQuery::new(min, max);
        Self {
            cache_key: query.cache_key(),
            query_type: QueryType::Box(query),
        }
    }
    
    pub fn query_type(&self) -> &QueryType {
        &self.query_type
    }
    
    pub fn cache_key(&self) -> u64 {
        self.cache_key
    }
}

impl Hash for SpatialQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cache_key.hash(state);
    }
}

impl PartialEq for SpatialQuery {
    fn eq(&self, other: &Self) -> bool {
        self.cache_key == other.cache_key
    }
}

impl Eq for SpatialQuery {}

/// Query for entities within a radius
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeQuery {
    center: [f32; 3],
    radius: f32,
    entity_type: Option<EntityType>,
}

impl RangeQuery {
    pub fn new(center: [f32; 3], radius: f32) -> Self {
        Self {
            center,
            radius,
            entity_type: None,
        }
    }
    
    pub fn with_type(mut self, entity_type: EntityType) -> Self {
        self.entity_type = Some(entity_type);
        self
    }
    
    pub fn center(&self) -> [f32; 3] {
        self.center
    }
    
    pub fn radius(&self) -> f32 {
        self.radius
    }
    
    pub fn entity_type(&self) -> Option<EntityType> {
        self.entity_type
    }
    
    fn cache_key(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        
        // Hash components
        for &v in &self.center {
            hasher.write_u32(v.to_bits());
        }
        hasher.write_u32(self.radius.to_bits());
        if let Some(et) = self.entity_type {
            hasher.write_u8(et as u8);
        }
        
        hasher.finish()
    }
}

/// Query for k nearest neighbors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KNearestQuery {
    center: [f32; 3],
    k: usize,
    max_distance: Option<f32>,
    entity_type: Option<EntityType>,
}

impl KNearestQuery {
    pub fn new(center: [f32; 3], k: usize) -> Self {
        Self {
            center,
            k,
            max_distance: None,
            entity_type: None,
        }
    }
    
    pub fn with_max_distance(mut self, distance: f32) -> Self {
        self.max_distance = Some(distance);
        self
    }
    
    pub fn with_type(mut self, entity_type: EntityType) -> Self {
        self.entity_type = Some(entity_type);
        self
    }
    
    pub fn center(&self) -> [f32; 3] {
        self.center
    }
    
    pub fn k(&self) -> usize {
        self.k
    }
    
    pub fn max_distance(&self) -> Option<f32> {
        self.max_distance
    }
    
    pub fn entity_type(&self) -> Option<EntityType> {
        self.entity_type
    }
    
    fn cache_key(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        
        for &v in &self.center {
            hasher.write_u32(v.to_bits());
        }
        hasher.write_usize(self.k);
        if let Some(d) = self.max_distance {
            hasher.write_u32(d.to_bits());
        }
        if let Some(et) = self.entity_type {
            hasher.write_u8(et as u8);
        }
        
        hasher.finish()
    }
}

/// View frustum for culling queries
#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    pub planes: [Plane; 6], // Near, far, left, right, top, bottom
}

impl Frustum {
    pub fn new(planes: [Plane; 6]) -> Self {
        Self { planes }
    }
    
    pub fn from_view_projection(view_proj: [[f32; 4]; 4]) -> Self {
        // Extract frustum planes from view-projection matrix
        let mut planes = [Plane::default(); 6];
        
        // Left plane
        planes[0] = Plane::new(
            view_proj[0][3] + view_proj[0][0],
            view_proj[1][3] + view_proj[1][0],
            view_proj[2][3] + view_proj[2][0],
            view_proj[3][3] + view_proj[3][0],
        );
        
        // Right plane
        planes[1] = Plane::new(
            view_proj[0][3] - view_proj[0][0],
            view_proj[1][3] - view_proj[1][0],
            view_proj[2][3] - view_proj[2][0],
            view_proj[3][3] - view_proj[3][0],
        );
        
        // Bottom plane
        planes[2] = Plane::new(
            view_proj[0][3] + view_proj[0][1],
            view_proj[1][3] + view_proj[1][1],
            view_proj[2][3] + view_proj[2][1],
            view_proj[3][3] + view_proj[3][1],
        );
        
        // Top plane
        planes[3] = Plane::new(
            view_proj[0][3] - view_proj[0][1],
            view_proj[1][3] - view_proj[1][1],
            view_proj[2][3] - view_proj[2][1],
            view_proj[3][3] - view_proj[3][1],
        );
        
        // Near plane
        planes[4] = Plane::new(
            view_proj[0][3] + view_proj[0][2],
            view_proj[1][3] + view_proj[1][2],
            view_proj[2][3] + view_proj[2][2],
            view_proj[3][3] + view_proj[3][2],
        );
        
        // Far plane
        planes[5] = Plane::new(
            view_proj[0][3] - view_proj[0][2],
            view_proj[1][3] - view_proj[1][2],
            view_proj[2][3] - view_proj[2][2],
            view_proj[3][3] - view_proj[3][2],
        );
        
        // Normalize planes
        for plane in &mut planes {
            plane.normalize();
        }
        
        Self { planes }
    }
    
    pub fn contains_sphere(&self, center: [f32; 3], radius: f32) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(center) < -radius {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Plane {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
}

impl Plane {
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
        Self { a, b, c, d }
    }
    
    pub fn normalize(&mut self) {
        let len = (self.a * self.a + self.b * self.b + self.c * self.c).sqrt();
        if len > 0.0 {
            self.a /= len;
            self.b /= len;
            self.c /= len;
            self.d /= len;
        }
    }
    
    pub fn distance_to_point(&self, point: [f32; 3]) -> f32 {
        self.a * point[0] + self.b * point[1] + self.c * point[2] + self.d
    }
}

/// Query for entities in a frustum
#[derive(Debug, Clone)]
pub struct FrustumQuery {
    frustum: Frustum,
    entity_type: Option<EntityType>,
}

impl FrustumQuery {
    pub fn new(frustum: Frustum) -> Self {
        Self {
            frustum,
            entity_type: None,
        }
    }
    
    pub fn with_type(mut self, entity_type: EntityType) -> Self {
        self.entity_type = Some(entity_type);
        self
    }
    
    pub fn frustum(&self) -> &Frustum {
        &self.frustum
    }
    
    pub fn entity_type(&self) -> Option<EntityType> {
        self.entity_type
    }
    
    fn cache_key(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        
        // Hash frustum planes
        for plane in &self.frustum.planes {
            hasher.write_u32(plane.a.to_bits());
            hasher.write_u32(plane.b.to_bits());
            hasher.write_u32(plane.c.to_bits());
            hasher.write_u32(plane.d.to_bits());
        }
        
        if let Some(et) = self.entity_type {
            hasher.write_u8(et as u8);
        }
        
        hasher.finish()
    }
}

/// Query for entities in an axis-aligned box
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoxQuery {
    min: [f32; 3],
    max: [f32; 3],
    entity_type: Option<EntityType>,
}

impl BoxQuery {
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self {
            min,
            max,
            entity_type: None,
        }
    }
    
    pub fn with_type(mut self, entity_type: EntityType) -> Self {
        self.entity_type = Some(entity_type);
        self
    }
    
    pub fn min(&self) -> [f32; 3] {
        self.min
    }
    
    pub fn max(&self) -> [f32; 3] {
        self.max
    }
    
    pub fn entity_type(&self) -> Option<EntityType> {
        self.entity_type
    }
    
    fn cache_key(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        
        for &v in &self.min {
            hasher.write_u32(v.to_bits());
        }
        for &v in &self.max {
            hasher.write_u32(v.to_bits());
        }
        if let Some(et) = self.entity_type {
            hasher.write_u8(et as u8);
        }
        
        hasher.finish()
    }
}