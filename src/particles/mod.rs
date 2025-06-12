// Legacy OOP modules (to be removed)
pub mod particle;
pub mod emitter;
pub mod particle_system;
pub mod effects;
pub mod physics;

// New data-oriented modules
pub mod particle_data;
pub mod update;
pub mod system;

// Re-export legacy types (for compatibility during transition)
pub use particle::{Particle, ParticleType, ParticleProperties};
pub use emitter::{ParticleEmitter, EmitterShape, EmissionPattern};
pub use particle_system::{ParticleSystem, ParticleUpdate};
pub use effects::{ParticleEffect, EffectPreset};
pub use physics::{ParticlePhysics, ParticleCollision};

// Export new data-oriented types
pub use particle_data::{ParticleData, EmitterData, ParticlePool, ParticleGPUData, MAX_PARTICLES};
pub use system::{DOPParticleSystem, ParticleStats};
pub use update::{update_particles, update_emitters, spawn_particle};