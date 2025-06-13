// Data-oriented ECS modules
pub mod entity_data;
pub mod component_data;
pub mod world_data;
pub mod systems_data;

// New SOA implementation
pub mod soa_world;

// Compatibility module
pub mod components;

// Re-export main types
pub use entity_data::{EntityId, EntityData, MAX_ENTITIES};
pub use component_data::{
    ComponentData, 
    TransformData, 
    PhysicsData, 
    ItemData, 
    RenderableData,
    COMPONENT_TRANSFORM,
    COMPONENT_PHYSICS,
    COMPONENT_ITEM,
    COMPONENT_RENDERABLE,
};
pub use world_data::EcsWorldData;

// Re-export SOA types
pub use soa_world::{
    SoAWorld,
    TransformSoA,
    PhysicsSoA,
    RenderableSoA,
    ComponentType,
    add_transform_component,
    add_physics_component,
    add_renderable_component,
    remove_transform_component,
    remove_physics_component,
    remove_renderable_component,
    update_physics_system as soa_update_physics_system,
    update_culling_system,
};

// Compatibility aliases
pub use world_data::EcsWorldData as EcsWorld;
pub use entity_data::EntityId as Entity;

// Re-export system functions
pub use systems_data::{
    update_physics_system,
    update_transform_from_physics,
    update_item_lifetimes,
    update_item_physics_system,
    check_item_pickups,
    spawn_dropped_item,
    apply_impulse,
    set_velocity,
    get_position,
    get_velocity,
};

// Re-export world helper functions
pub use world_data::{
    get_transform,
    get_transform_mut,
    get_physics,
    get_physics_mut,
    has_transform,
    has_physics,
    has_item,
    spawn_item,
};