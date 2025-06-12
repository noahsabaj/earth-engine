/// Compatibility module for component types
/// Maps old OOP-style components to data-oriented versions

use cgmath::Vector3;

/// Transform component for compatibility
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Vector3<f32>,
    pub scale: Vector3<f32>,
}

/// Physics component for compatibility
#[derive(Debug, Clone, Copy)]
pub struct Physics {
    pub velocity: Vector3<f32>,
    pub acceleration: Vector3<f32>,
    pub mass: f32,
    pub grounded: bool,
}

/// Item component for compatibility
#[derive(Debug, Clone)]
pub struct Item {
    pub item_id: u32,
    pub count: u32,
    pub lifetime: f32,
}

/// Renderable component for compatibility
#[derive(Debug, Clone)]
pub struct Renderable {
    pub mesh_id: u32,
    pub material_id: u32,
    pub visible: bool,
}