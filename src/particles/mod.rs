// OOP modules (to be migrated to DOP)
pub mod particle;
pub mod emitter;
pub mod particle_system;
pub mod effects;
pub mod physics;

// Data-oriented modules
pub mod particle_data;
pub mod update;
pub mod system;
pub mod gpu_particle_system;

// Re-export types
pub use particle::{Particle, ParticleType, ParticleProperties};
pub use emitter::{ParticleEmitter, EmitterShape, EmissionPattern};
pub use particle_system::{ParticleSystem, ParticleUpdate};
pub use effects::{ParticleEffect, EffectPreset};
pub use physics::{ParticlePhysics, ParticleCollision};

// Export new data-oriented types
pub use particle_data::{ParticleData, EmitterData, ParticlePool, ParticleGPUData, MAX_PARTICLES};
pub use system::{DOPParticleSystem, ParticleStats};
pub use update::{update_particles, update_emitters, spawn_particle};
pub use gpu_particle_system::GpuParticleSystem;