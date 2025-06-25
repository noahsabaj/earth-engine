// Data-oriented modules
pub mod dop_system_operations;
pub mod emitter_data;
pub mod emitter_operations;
pub mod effects_data;
pub mod effects_operations;
pub mod gpu_particle_system;
pub mod particle_data;
pub mod particle_operations;
pub mod particle_system_data;
pub mod particle_system_operations;
pub mod particle_types;
pub mod physics_data;
pub mod physics_operations;
pub mod system_data;
pub mod update;

// Re-export data types
pub use effects_data::{EffectPreset, ParticleEffectData};
pub use emitter_data::{EmissionPattern, EmitterShape, ParticleEmitterData, create_default_emitter, create_fire_emitter, create_smoke_emitter, create_magic_emitter};
pub use particle_system_data::{ParticleSystemData, ParticleUpdateData};
pub use particle_types::{ColorCurve, Particle, ParticleProperties, ParticleType, SizeCurve, particle_type_to_id, create_default_particle_properties};
pub use physics_data::{ParticleCollisionData, ParticlePhysicsData};

// Re-export operations
pub use effects_operations::{create_effect_from_preset, update_effect};
pub use emitter_operations::{update_emitter};
pub use particle_operations::{create_particle, update_particle};
pub use particle_system_operations::{create_particle_system, update_particle_system};
pub use physics_operations::{create_physics_data, update_particle_physics};

// Export existing data-oriented types
pub use dop_system_operations::*;
pub use gpu_particle_system::GpuParticleSystem;
pub use particle_data::{EmitterData, ParticleData, ParticleGPUData, ParticlePool, MAX_PARTICLES, create_particle_data, create_emitter_data, clear_particle_data, clear_emitter_data, remove_particle_swap};
pub use system_data::{DOPParticleSystem, ParticleStats};
pub use update::{spawn_particle, update_emitters, update_particles};

// Compatibility re-exports (temporary, can be removed after full migration)
pub use particle_system_data::ParticleUpdateData as ParticleUpdate;
pub use particle_system_data::ParticleSystemData as ParticleSystem;
pub use emitter_data::ParticleEmitterData as ParticleEmitter;
pub use effects_data::ParticleEffectData as ParticleEffect;
pub use physics_data::ParticlePhysicsData as ParticlePhysics;
pub use physics_data::ParticleCollisionData as ParticleCollision;
