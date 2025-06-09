use cgmath::{Vector3, Point3};
use crate::{BlockId, AABB};
use crate::item::ItemId;

/// Transform component for position and rotation
#[derive(Debug, Clone)]
pub struct Transform {
    pub position: Point3<f32>,
    pub rotation: Vector3<f32>, // Euler angles
    pub scale: Vector3<f32>,
}

impl Transform {
    pub fn new(position: Point3<f32>) -> Self {
        Self {
            position,
            rotation: Vector3::new(0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

/// Physics component for entities that need physics simulation
#[derive(Debug, Clone)]
pub struct Physics {
    pub velocity: Vector3<f32>,
    pub acceleration: Vector3<f32>,
    pub mass: f32,
    pub gravity_scale: f32,
    pub drag: f32,
    pub angular_velocity: Vector3<f32>,
    pub bounding_box: AABB,
    pub grounded: bool,
}

impl Physics {
    pub fn new(mass: f32, bounding_box: AABB) -> Self {
        Self {
            velocity: Vector3::new(0.0, 0.0, 0.0),
            acceleration: Vector3::new(0.0, 0.0, 0.0),
            mass,
            gravity_scale: 1.0,
            drag: 0.1,
            angular_velocity: Vector3::new(0.0, 0.0, 0.0),
            bounding_box,
            grounded: false,
        }
    }
}

/// Item component for dropped items in the world
#[derive(Debug, Clone)]
pub struct ItemComponent {
    pub item_id: ItemId,
    pub stack_size: u32,
    pub pickup_delay: f32, // Delay before item can be picked up
    pub lifetime: f32, // How long the item exists before despawning
}

impl ItemComponent {
    pub fn new(item_id: ItemId, stack_size: u32) -> Self {
        Self {
            item_id,
            stack_size,
            pickup_delay: 0.5, // Half second delay
            lifetime: 300.0, // 5 minutes
        }
    }
}

/// Renderable component for entities that should be rendered
#[derive(Debug, Clone)]
pub struct Renderable {
    pub model_type: ModelType,
    pub color: [f32; 3],
    pub scale: f32,
}

#[derive(Debug, Clone)]
pub enum ModelType {
    Cube,
    Item(BlockId),
    // Add more model types as needed
}

impl Renderable {
    pub fn cube(color: [f32; 3], scale: f32) -> Self {
        Self {
            model_type: ModelType::Cube,
            color,
            scale,
        }
    }
    
    pub fn item(block_id: BlockId, scale: f32) -> Self {
        Self {
            model_type: ModelType::Item(block_id),
            color: [1.0, 1.0, 1.0],
            scale,
        }
    }
}