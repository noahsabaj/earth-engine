/// Compatibility module for physics body types
/// Maps to data-oriented physics system

// Re-export PhysicsData with compatibility aliases
pub type PhysicsBody = super::PhysicsData;
pub type RigidBody = super::PhysicsData;
pub type PlayerBody = super::PhysicsData;

// Re-export MovementState
pub use crate::network::MovementState;