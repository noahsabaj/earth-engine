// OOP modules (to be migrated to DOP)
pub mod effects;
pub mod emitter;
pub mod particle;
pub mod particle_system;
pub mod physics;

// Data-oriented modules
pub mod gpu_particle_system;
pub mod particle_data;
pub mod system;
pub mod update;

// Re-export types
pub use effects::{EffectPreset, ParticleEffect};
pub use emitter::{EmissionPattern, EmitterShape, ParticleEmitter};
pub use particle::{Particle, ParticleProperties, ParticleType};
pub use particle_system::{ParticleSystem, ParticleUpdate};
pub use physics::{ParticleCollision, ParticlePhysics};

// Export new data-oriented types
pub use gpu_particle_system::GpuParticleSystem;
pub use particle_data::{EmitterData, ParticleData, ParticleGPUData, ParticlePool, MAX_PARTICLES};
pub use system::{DOPParticleSystem, ParticleStats};
pub use update::{spawn_particle, update_emitters, update_particles};
