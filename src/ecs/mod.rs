pub mod entity;
pub mod component;
pub mod world;
pub mod system;

pub use entity::{Entity, EntityManager};
pub use component::{Component, ComponentStorage, AnyComponentStorage};
pub use world::EcsWorld;
pub use system::System;

// Common components
pub mod components;

// Systems
pub mod systems;

/// Re-export commonly used types
pub use components::{
    Transform,
    Physics,
    ItemComponent,
    Renderable,
};