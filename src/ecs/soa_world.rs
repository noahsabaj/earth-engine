/// Comprehensive SOA Entity World
/// 
/// This module implements a complete Structure-of-Arrays ECS system
/// that provides better cache efficiency than the current sparse set approach.

use super::entity_data::{EntityId, MAX_ENTITIES};
use std::sync::atomic::{AtomicU32, Ordering};

/// Maximum components of any single type
pub const MAX_COMPONENTS: usize = MAX_ENTITIES;

/// Component type identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ComponentType {
    Transform = 0,
    Physics = 1,
    Renderable = 2,
    Item = 3,
}

impl ComponentType {
    pub fn bit_mask(self) -> u64 {
        1u64 << (self as u8)
    }
}

/// Transform component data in SOA layout
#[derive(Debug)]
pub struct TransformSoA {
    /// Number of active transform components
    pub count: AtomicU32,
    
    /// Position data (cache-friendly)
    pub positions_x: Vec<f32>,
    pub positions_y: Vec<f32>,
    pub positions_z: Vec<f32>,
    
    /// Rotation data (Euler angles)
    pub rotations_x: Vec<f32>,
    pub rotations_y: Vec<f32>,
    pub rotations_z: Vec<f32>,
    
    /// Scale data
    pub scales_x: Vec<f32>,
    pub scales_y: Vec<f32>,
    pub scales_z: Vec<f32>,
    
    /// Mapping: component index -> entity
    pub entities: Vec<EntityId>,
    
    /// Mapping: entity index -> component index (sparse)
    pub entity_to_component: Vec<Option<u32>>,
}

impl TransformSoA {
    pub fn new() -> Self {
        Self {
            count: AtomicU32::new(0),
            positions_x: Vec::with_capacity(MAX_COMPONENTS),
            positions_y: Vec::with_capacity(MAX_COMPONENTS),
            positions_z: Vec::with_capacity(MAX_COMPONENTS),
            rotations_x: Vec::with_capacity(MAX_COMPONENTS),
            rotations_y: Vec::with_capacity(MAX_COMPONENTS),
            rotations_z: Vec::with_capacity(MAX_COMPONENTS),
            scales_x: Vec::with_capacity(MAX_COMPONENTS),
            scales_y: Vec::with_capacity(MAX_COMPONENTS),
            scales_z: Vec::with_capacity(MAX_COMPONENTS),
            entities: Vec::with_capacity(MAX_COMPONENTS),
            entity_to_component: vec![None; MAX_ENTITIES],
        }
    }
    
    /// Get current component count
    pub fn len(&self) -> usize {
        self.count.load(Ordering::Acquire) as usize
    }
    
    /// Clear all components
    pub fn clear(&mut self) {
        self.count.store(0, Ordering::Release);
        self.positions_x.clear();
        self.positions_y.clear();
        self.positions_z.clear();
        self.rotations_x.clear();
        self.rotations_y.clear();
        self.rotations_z.clear();
        self.scales_x.clear();
        self.scales_y.clear();
        self.scales_z.clear();
        self.entities.clear();
        self.entity_to_component.fill(None);
    }
}

/// Physics component data in SOA layout
#[derive(Debug)]
pub struct PhysicsSoA {
    /// Number of active physics components
    pub count: AtomicU32,
    
    /// Velocity data
    pub velocities_x: Vec<f32>,
    pub velocities_y: Vec<f32>,
    pub velocities_z: Vec<f32>,
    
    /// Acceleration data
    pub accelerations_x: Vec<f32>,
    pub accelerations_y: Vec<f32>,
    pub accelerations_z: Vec<f32>,
    
    /// Angular velocity data
    pub angular_velocities_x: Vec<f32>,
    pub angular_velocities_y: Vec<f32>,
    pub angular_velocities_z: Vec<f32>,
    
    /// Mass data
    pub masses: Vec<f32>,
    pub inverse_masses: Vec<f32>,
    pub gravity_scales: Vec<f32>,
    pub drags: Vec<f32>,
    
    /// AABB data
    pub aabb_min_x: Vec<f32>,
    pub aabb_min_y: Vec<f32>,
    pub aabb_min_z: Vec<f32>,
    pub aabb_max_x: Vec<f32>,
    pub aabb_max_y: Vec<f32>,
    pub aabb_max_z: Vec<f32>,
    
    /// Status flags
    pub grounded: Vec<bool>,
    
    /// Mapping data
    pub entities: Vec<EntityId>,
    pub entity_to_component: Vec<Option<u32>>,
}

impl PhysicsSoA {
    pub fn new() -> Self {
        Self {
            count: AtomicU32::new(0),
            velocities_x: Vec::with_capacity(MAX_COMPONENTS),
            velocities_y: Vec::with_capacity(MAX_COMPONENTS),
            velocities_z: Vec::with_capacity(MAX_COMPONENTS),
            accelerations_x: Vec::with_capacity(MAX_COMPONENTS),
            accelerations_y: Vec::with_capacity(MAX_COMPONENTS),
            accelerations_z: Vec::with_capacity(MAX_COMPONENTS),
            angular_velocities_x: Vec::with_capacity(MAX_COMPONENTS),
            angular_velocities_y: Vec::with_capacity(MAX_COMPONENTS),
            angular_velocities_z: Vec::with_capacity(MAX_COMPONENTS),
            masses: Vec::with_capacity(MAX_COMPONENTS),
            inverse_masses: Vec::with_capacity(MAX_COMPONENTS),
            gravity_scales: Vec::with_capacity(MAX_COMPONENTS),
            drags: Vec::with_capacity(MAX_COMPONENTS),
            aabb_min_x: Vec::with_capacity(MAX_COMPONENTS),
            aabb_min_y: Vec::with_capacity(MAX_COMPONENTS),
            aabb_min_z: Vec::with_capacity(MAX_COMPONENTS),
            aabb_max_x: Vec::with_capacity(MAX_COMPONENTS),
            aabb_max_y: Vec::with_capacity(MAX_COMPONENTS),
            aabb_max_z: Vec::with_capacity(MAX_COMPONENTS),
            grounded: Vec::with_capacity(MAX_COMPONENTS),
            entities: Vec::with_capacity(MAX_COMPONENTS),
            entity_to_component: vec![None; MAX_ENTITIES],
        }
    }
    
    pub fn len(&self) -> usize {
        self.count.load(Ordering::Acquire) as usize
    }
    
    pub fn clear(&mut self) {
        self.count.store(0, Ordering::Release);
        self.velocities_x.clear();
        self.velocities_y.clear();
        self.velocities_z.clear();
        self.accelerations_x.clear();
        self.accelerations_y.clear();
        self.accelerations_z.clear();
        self.angular_velocities_x.clear();
        self.angular_velocities_y.clear();
        self.angular_velocities_z.clear();
        self.masses.clear();
        self.inverse_masses.clear();
        self.gravity_scales.clear();
        self.drags.clear();
        self.aabb_min_x.clear();
        self.aabb_min_y.clear();
        self.aabb_min_z.clear();
        self.aabb_max_x.clear();
        self.aabb_max_y.clear();
        self.aabb_max_z.clear();
        self.grounded.clear();
        self.entities.clear();
        self.entity_to_component.fill(None);
    }
}

/// Renderable component data in SOA layout
#[derive(Debug)]
pub struct RenderableSoA {
    pub count: AtomicU32,
    
    /// Rendering data
    pub model_types: Vec<u32>,
    pub model_data: Vec<u32>,
    pub colors_r: Vec<f32>,
    pub colors_g: Vec<f32>,
    pub colors_b: Vec<f32>,
    pub scales: Vec<f32>,
    pub visible: Vec<bool>,
    
    /// Mapping data
    pub entities: Vec<EntityId>,
    pub entity_to_component: Vec<Option<u32>>,
}

impl RenderableSoA {
    pub fn new() -> Self {
        Self {
            count: AtomicU32::new(0),
            model_types: Vec::with_capacity(MAX_COMPONENTS),
            model_data: Vec::with_capacity(MAX_COMPONENTS),
            colors_r: Vec::with_capacity(MAX_COMPONENTS),
            colors_g: Vec::with_capacity(MAX_COMPONENTS),
            colors_b: Vec::with_capacity(MAX_COMPONENTS),
            scales: Vec::with_capacity(MAX_COMPONENTS),
            visible: Vec::with_capacity(MAX_COMPONENTS),
            entities: Vec::with_capacity(MAX_COMPONENTS),
            entity_to_component: vec![None; MAX_ENTITIES],
        }
    }
    
    pub fn len(&self) -> usize {
        self.count.load(Ordering::Acquire) as usize
    }
    
    pub fn clear(&mut self) {
        self.count.store(0, Ordering::Release);
        self.model_types.clear();
        self.model_data.clear();
        self.colors_r.clear();
        self.colors_g.clear();
        self.colors_b.clear();
        self.scales.clear();
        self.visible.clear();
        self.entities.clear();
        self.entity_to_component.fill(None);
    }
}

/// Complete SOA ECS World
pub struct SoAWorld {
    /// Entity management
    pub entities: super::entity_data::EntityData,
    
    /// Component storage
    pub transforms: TransformSoA,
    pub physics: PhysicsSoA,
    pub renderables: RenderableSoA,
}

impl SoAWorld {
    pub fn new() -> Self {
        Self {
            entities: super::entity_data::EntityData::new(),
            transforms: TransformSoA::new(),
            physics: PhysicsSoA::new(),
            renderables: RenderableSoA::new(),
        }
    }
    
    /// Create a new entity
    pub fn create_entity(&mut self) -> EntityId {
        self.entities.create()
    }
    
    /// Destroy an entity and all its components
    pub fn destroy_entity(&mut self, entity: EntityId) -> bool {
        if !self.entities.destroy(entity) {
            return false;
        }
        
        // Remove from all component arrays
        remove_transform_component(&mut self.transforms, entity);
        remove_physics_component(&mut self.physics, entity);
        remove_renderable_component(&mut self.renderables, entity);
        
        true
    }
    
    /// Clear all entities and components
    pub fn clear(&mut self) {
        self.entities.clear();
        self.transforms.clear();
        self.physics.clear();
        self.renderables.clear();
    }
}

// Pure functions for component operations

/// Add transform component to entity
pub fn add_transform_component(
    transforms: &mut TransformSoA,
    entities: &mut super::entity_data::EntityData,
    entity: EntityId,
    position: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3],
) -> bool {
    if !entity.is_valid() || transforms.len() >= MAX_COMPONENTS {
        return false;
    }
    
    let entity_idx = entity.idx();
    
    // Check if entity already has this component
    if transforms.entity_to_component[entity_idx].is_some() {
        return false;
    }
    
    let component_idx = transforms.len() as u32;
    
    // Add component data
    transforms.positions_x.push(position[0]);
    transforms.positions_y.push(position[1]);
    transforms.positions_z.push(position[2]);
    transforms.rotations_x.push(rotation[0]);
    transforms.rotations_y.push(rotation[1]);
    transforms.rotations_z.push(rotation[2]);
    transforms.scales_x.push(scale[0]);
    transforms.scales_y.push(scale[1]);
    transforms.scales_z.push(scale[2]);
    transforms.entities.push(entity);
    
    // Update mappings
    transforms.entity_to_component[entity_idx] = Some(component_idx);
    transforms.count.fetch_add(1, Ordering::AcqRel);
    
    // Update entity component mask
    entities.set_component_bit(entity, ComponentType::Transform as u8);
    
    true
}

/// Remove transform component from entity
pub fn remove_transform_component(transforms: &mut TransformSoA, entity: EntityId) -> bool {
    if !entity.is_valid() {
        return false;
    }
    
    let entity_idx = entity.idx();
    
    if let Some(component_idx) = transforms.entity_to_component[entity_idx] {
        let component_idx = component_idx as usize;
        let last_idx = transforms.len() - 1;
        
        if component_idx != last_idx {
            // Swap with last element
            transforms.positions_x.swap(component_idx, last_idx);
            transforms.positions_y.swap(component_idx, last_idx);
            transforms.positions_z.swap(component_idx, last_idx);
            transforms.rotations_x.swap(component_idx, last_idx);
            transforms.rotations_y.swap(component_idx, last_idx);
            transforms.rotations_z.swap(component_idx, last_idx);
            transforms.scales_x.swap(component_idx, last_idx);
            transforms.scales_y.swap(component_idx, last_idx);
            transforms.scales_z.swap(component_idx, last_idx);
            transforms.entities.swap(component_idx, last_idx);
            
            // Update moved entity's mapping
            let moved_entity = transforms.entities[component_idx];
            if moved_entity.is_valid() {
                transforms.entity_to_component[moved_entity.idx()] = Some(component_idx as u32);
            }
        }
        
        // Remove last element
        transforms.positions_x.pop();
        transforms.positions_y.pop();
        transforms.positions_z.pop();
        transforms.rotations_x.pop();
        transforms.rotations_y.pop();
        transforms.rotations_z.pop();
        transforms.scales_x.pop();
        transforms.scales_y.pop();
        transforms.scales_z.pop();
        transforms.entities.pop();
        
        // Clear mapping
        transforms.entity_to_component[entity_idx] = None;
        transforms.count.fetch_sub(1, Ordering::AcqRel);
        
        true
    } else {
        false
    }
}

/// Add physics component to entity
pub fn add_physics_component(
    physics: &mut PhysicsSoA,
    entities: &mut super::entity_data::EntityData,
    entity: EntityId,
    velocity: [f32; 3],
    mass: f32,
) -> bool {
    if !entity.is_valid() || physics.len() >= MAX_COMPONENTS {
        return false;
    }
    
    let entity_idx = entity.idx();
    
    if physics.entity_to_component[entity_idx].is_some() {
        return false;
    }
    
    let component_idx = physics.len() as u32;
    
    // Add component data
    physics.velocities_x.push(velocity[0]);
    physics.velocities_y.push(velocity[1]);
    physics.velocities_z.push(velocity[2]);
    physics.accelerations_x.push(0.0);
    physics.accelerations_y.push(0.0);
    physics.accelerations_z.push(0.0);
    physics.angular_velocities_x.push(0.0);
    physics.angular_velocities_y.push(0.0);
    physics.angular_velocities_z.push(0.0);
    physics.masses.push(mass);
    physics.inverse_masses.push(if mass > 0.0 { 1.0 / mass } else { 0.0 });
    physics.gravity_scales.push(1.0);
    physics.drags.push(0.1);
    physics.aabb_min_x.push(-0.5);
    physics.aabb_min_y.push(-0.5);
    physics.aabb_min_z.push(-0.5);
    physics.aabb_max_x.push(0.5);
    physics.aabb_max_y.push(0.5);
    physics.aabb_max_z.push(0.5);
    physics.grounded.push(false);
    physics.entities.push(entity);
    
    // Update mappings
    physics.entity_to_component[entity_idx] = Some(component_idx);
    physics.count.fetch_add(1, Ordering::AcqRel);
    
    // Update entity component mask
    entities.set_component_bit(entity, ComponentType::Physics as u8);
    
    true
}

/// Remove physics component from entity
pub fn remove_physics_component(physics: &mut PhysicsSoA, entity: EntityId) -> bool {
    if !entity.is_valid() {
        return false;
    }
    
    let entity_idx = entity.idx();
    
    if let Some(component_idx) = physics.entity_to_component[entity_idx] {
        let component_idx = component_idx as usize;
        let last_idx = physics.len() - 1;
        
        if component_idx != last_idx {
            // Swap with last element (all arrays)
            physics.velocities_x.swap(component_idx, last_idx);
            physics.velocities_y.swap(component_idx, last_idx);
            physics.velocities_z.swap(component_idx, last_idx);
            physics.accelerations_x.swap(component_idx, last_idx);
            physics.accelerations_y.swap(component_idx, last_idx);
            physics.accelerations_z.swap(component_idx, last_idx);
            physics.angular_velocities_x.swap(component_idx, last_idx);
            physics.angular_velocities_y.swap(component_idx, last_idx);
            physics.angular_velocities_z.swap(component_idx, last_idx);
            physics.masses.swap(component_idx, last_idx);
            physics.inverse_masses.swap(component_idx, last_idx);
            physics.gravity_scales.swap(component_idx, last_idx);
            physics.drags.swap(component_idx, last_idx);
            physics.aabb_min_x.swap(component_idx, last_idx);
            physics.aabb_min_y.swap(component_idx, last_idx);
            physics.aabb_min_z.swap(component_idx, last_idx);
            physics.aabb_max_x.swap(component_idx, last_idx);
            physics.aabb_max_y.swap(component_idx, last_idx);
            physics.aabb_max_z.swap(component_idx, last_idx);
            physics.grounded.swap(component_idx, last_idx);
            physics.entities.swap(component_idx, last_idx);
            
            // Update moved entity's mapping
            let moved_entity = physics.entities[component_idx];
            if moved_entity.is_valid() {
                physics.entity_to_component[moved_entity.idx()] = Some(component_idx as u32);
            }
        }
        
        // Remove last element from all arrays
        physics.velocities_x.pop();
        physics.velocities_y.pop();
        physics.velocities_z.pop();
        physics.accelerations_x.pop();
        physics.accelerations_y.pop();
        physics.accelerations_z.pop();
        physics.angular_velocities_x.pop();
        physics.angular_velocities_y.pop();
        physics.angular_velocities_z.pop();
        physics.masses.pop();
        physics.inverse_masses.pop();
        physics.gravity_scales.pop();
        physics.drags.pop();
        physics.aabb_min_x.pop();
        physics.aabb_min_y.pop();
        physics.aabb_min_z.pop();
        physics.aabb_max_x.pop();
        physics.aabb_max_y.pop();
        physics.aabb_max_z.pop();
        physics.grounded.pop();
        physics.entities.pop();
        
        physics.entity_to_component[entity_idx] = None;
        physics.count.fetch_sub(1, Ordering::AcqRel);
        
        true
    } else {
        false
    }
}

/// Add renderable component to entity
pub fn add_renderable_component(
    renderables: &mut RenderableSoA,
    entities: &mut super::entity_data::EntityData,
    entity: EntityId,
    model_type: u32,
    color: [f32; 3],
) -> bool {
    if !entity.is_valid() || renderables.len() >= MAX_COMPONENTS {
        return false;
    }
    
    let entity_idx = entity.idx();
    
    if renderables.entity_to_component[entity_idx].is_some() {
        return false;
    }
    
    let component_idx = renderables.len() as u32;
    
    // Add component data
    renderables.model_types.push(model_type);
    renderables.model_data.push(0);
    renderables.colors_r.push(color[0]);
    renderables.colors_g.push(color[1]);
    renderables.colors_b.push(color[2]);
    renderables.scales.push(1.0);
    renderables.visible.push(true);
    renderables.entities.push(entity);
    
    // Update mappings
    renderables.entity_to_component[entity_idx] = Some(component_idx);
    renderables.count.fetch_add(1, Ordering::AcqRel);
    
    // Update entity component mask
    entities.set_component_bit(entity, ComponentType::Renderable as u8);
    
    true
}

/// Remove renderable component from entity
pub fn remove_renderable_component(renderables: &mut RenderableSoA, entity: EntityId) -> bool {
    if !entity.is_valid() {
        return false;
    }
    
    let entity_idx = entity.idx();
    
    if let Some(component_idx) = renderables.entity_to_component[entity_idx] {
        let component_idx = component_idx as usize;
        let last_idx = renderables.len() - 1;
        
        if component_idx != last_idx {
            // Swap with last element
            renderables.model_types.swap(component_idx, last_idx);
            renderables.model_data.swap(component_idx, last_idx);
            renderables.colors_r.swap(component_idx, last_idx);
            renderables.colors_g.swap(component_idx, last_idx);
            renderables.colors_b.swap(component_idx, last_idx);
            renderables.scales.swap(component_idx, last_idx);
            renderables.visible.swap(component_idx, last_idx);
            renderables.entities.swap(component_idx, last_idx);
            
            // Update moved entity's mapping
            let moved_entity = renderables.entities[component_idx];
            if moved_entity.is_valid() {
                renderables.entity_to_component[moved_entity.idx()] = Some(component_idx as u32);
            }
        }
        
        // Remove last element
        renderables.model_types.pop();
        renderables.model_data.pop();
        renderables.colors_r.pop();
        renderables.colors_g.pop();
        renderables.colors_b.pop();
        renderables.scales.pop();
        renderables.visible.pop();
        renderables.entities.pop();
        
        renderables.entity_to_component[entity_idx] = None;
        renderables.count.fetch_sub(1, Ordering::AcqRel);
        
        true
    } else {
        false
    }
}

/// Physics update system - operates on raw SOA data for maximum cache efficiency
pub fn update_physics_system(
    transforms: &mut TransformSoA, 
    physics: &mut PhysicsSoA, 
    dt: f32
) {
    let count = physics.len();
    if count == 0 {
        return;
    }
    
    // Apply gravity to all entities (vectorizable)
    for i in 0..count {
        physics.accelerations_y[i] = -9.81 * physics.gravity_scales[i];
    }
    
    // Update velocities (vectorizable)
    for i in 0..count {
        physics.velocities_x[i] += physics.accelerations_x[i] * dt;
        physics.velocities_y[i] += physics.accelerations_y[i] * dt;
        physics.velocities_z[i] += physics.accelerations_z[i] * dt;
    }
    
    // Apply drag (vectorizable)
    for i in 0..count {
        let drag_factor = 1.0 - physics.drags[i] * dt;
        physics.velocities_x[i] *= drag_factor;
        physics.velocities_y[i] *= drag_factor;
        physics.velocities_z[i] *= drag_factor;
    }
    
    // Update positions for entities that have both transform and physics
    for i in 0..count {
        let entity = physics.entities[i];
        if let Some(transform_idx) = transforms.entity_to_component[entity.idx()] {
            let transform_idx = transform_idx as usize;
            if transform_idx < transforms.len() {
                transforms.positions_x[transform_idx] += physics.velocities_x[i] * dt;
                transforms.positions_y[transform_idx] += physics.velocities_y[i] * dt;
                transforms.positions_z[transform_idx] += physics.velocities_z[i] * dt;
            }
        }
    }
}

/// Rendering culling system - operates on SOA data for maximum performance
pub fn update_culling_system(
    transforms: &TransformSoA,
    renderables: &mut RenderableSoA,
    camera_position: [f32; 3],
    view_distance: f32,
) {
    let count = renderables.len();
    if count == 0 {
        return;
    }
    
    let view_distance_sq = view_distance * view_distance;
    
    // Frustum culling (vectorizable)
    for i in 0..count {
        let entity = renderables.entities[i];
        if let Some(transform_idx) = transforms.entity_to_component[entity.idx()] {
            let transform_idx = transform_idx as usize;
            if transform_idx < transforms.len() {
                let dx = transforms.positions_x[transform_idx] - camera_position[0];
                let dy = transforms.positions_y[transform_idx] - camera_position[1];
                let dz = transforms.positions_z[transform_idx] - camera_position[2];
                let distance_sq = dx * dx + dy * dy + dz * dz;
                
                renderables.visible[i] = distance_sq <= view_distance_sq;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_soa_world_creation() {
        let mut world = SoAWorld::new();
        let entity = world.create_entity();
        assert!(entity.is_valid());
        assert_eq!(world.entities.entity_count(), 1);
    }
    
    #[test]
    fn test_transform_component_soa() {
        let mut world = SoAWorld::new();
        let entity = world.create_entity();
        
        let result = add_transform_component(
            &mut world.transforms,
            &mut world.entities,
            entity,
            [1.0, 2.0, 3.0],
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
        );
        
        assert!(result);
        assert_eq!(world.transforms.len(), 1);
        assert_eq!(world.transforms.positions_x[0], 1.0);
        assert_eq!(world.transforms.positions_y[0], 2.0);
        assert_eq!(world.transforms.positions_z[0], 3.0);
    }
    
    #[test]
    fn test_physics_update_system() {
        let mut world = SoAWorld::new();
        let entity = world.create_entity();
        
        add_transform_component(
            &mut world.transforms,
            &mut world.entities,
            entity,
            [0.0, 10.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
        );
        
        add_physics_component(
            &mut world.physics,
            &mut world.entities,
            entity,
            [0.0, 0.0, 0.0],
            1.0,
        );
        
        let dt = 0.016; // 60 FPS
        update_physics_system(&mut world.transforms, &mut world.physics, dt);
        
        // Gravity should have affected velocity
        assert!(world.physics.velocities_y[0] < 0.0);
        // Position should have changed due to velocity
        assert!(world.transforms.positions_y[0] < 10.0);
    }
}