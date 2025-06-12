/// Compatibility module for physics body types
/// Maps to data-oriented PhysicsBodyData

// Re-export PhysicsBodyData with compatibility aliases
pub type PhysicsBody = super::PhysicsBodyData;
pub type RigidBody = super::PhysicsBodyData;
pub type PlayerBody = super::PhysicsBodyData;

// Re-export MovementState
pub use crate::network::MovementState;