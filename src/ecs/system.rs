use super::EcsWorld;

/// Trait for systems that process entities with specific components
pub trait System {
    /// Called once when the system is initialized
    fn init(&mut self, _world: &mut EcsWorld) {}
    
    /// Called every frame to update the system
    fn update(&mut self, world: &mut EcsWorld, delta_time: f32);
    
    /// Get the name of this system for debugging
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}