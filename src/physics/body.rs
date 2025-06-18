/// Compatibility module for physics body types
/// Maps to data-oriented physics_data system

// Re-export PhysicsData with compatibility aliases
pub type PhysicsBody = crate::physics_data::PhysicsData;
pub type RigidBody = crate::physics_data::PhysicsData;
pub type PlayerBody = crate::physics_data::PhysicsData;

// Re-export MovementState
pub use crate::network::MovementState;