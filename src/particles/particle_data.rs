/// Maximum number of particles that can exist
pub const MAX_PARTICLES: usize = 1_000_000;

/// Particle data stored in Structure of Arrays (SOA) layout for cache efficiency
pub struct ParticleData {
    /// Current number of active particles
    pub count: usize,

    /// Position buffers
    pub position_x: Vec<f32>,
    pub position_y: Vec<f32>,
    pub position_z: Vec<f32>,

    /// Velocity buffers
    pub velocity_x: Vec<f32>,
    pub velocity_y: Vec<f32>,
    pub velocity_z: Vec<f32>,

    /// Acceleration buffers
    pub acceleration_x: Vec<f32>,
    pub acceleration_y: Vec<f32>,
    pub acceleration_z: Vec<f32>,

    /// Color buffers (RGBA)
    pub color_r: Vec<f32>,
    pub color_g: Vec<f32>,
    pub color_b: Vec<f32>,
    pub color_a: Vec<f32>,

    /// Size buffer
    pub size: Vec<f32>,

    /// Lifetime buffers
    pub lifetime: Vec<f32>,
    pub max_lifetime: Vec<f32>,

    /// Particle type IDs
    pub particle_type: Vec<u32>,

    /// Physics properties
    pub gravity_multiplier: Vec<f32>,
    pub drag: Vec<f32>,
    pub bounce: Vec<f32>,

    /// Visual properties
    pub rotation: Vec<f32>,
    pub rotation_speed: Vec<f32>,
    pub texture_frame: Vec<u32>,
    pub animation_speed: Vec<f32>,
    pub emissive: Vec<bool>,
    pub emission_intensity: Vec<f32>,

    /// Size curve type (0=constant, 1=linear, 2=grow_shrink, 3=custom)
    pub size_curve_type: Vec<u8>,
    pub size_curve_param1: Vec<f32>,
    pub size_curve_param2: Vec<f32>,
    pub size_curve_param3: Vec<f32>,

    /// Color curve type (0=constant, 1=fadeout, 2=linear, 3=temperature, 4=custom)
    pub color_curve_type: Vec<u8>,
    pub color_curve_param1: Vec<f32>,
    pub color_curve_param2: Vec<f32>,
}

/// Create a new particle data buffer with pre-allocated capacity
pub fn create_particle_data(capacity: usize) -> ParticleData {
    // Safety check: prevent excessive memory allocations
    let safe_capacity = if capacity > MAX_PARTICLES {
        eprintln!(
            "WARNING: ParticleData capacity {} exceeds MAX_PARTICLES {}, clamping",
            capacity, MAX_PARTICLES
        );
        MAX_PARTICLES
    } else if capacity > 100_000_000 {
        // 100M particles would be ~13GB
        eprintln!(
            "WARNING: ParticleData capacity {} is extremely large, clamping to 100K",
            capacity
        );
        100_000
    } else {
        capacity
    };
    ParticleData {
            count: 0,

            position_x: Vec::with_capacity(safe_capacity),
            position_y: Vec::with_capacity(safe_capacity),
            position_z: Vec::with_capacity(safe_capacity),

            velocity_x: Vec::with_capacity(safe_capacity),
            velocity_y: Vec::with_capacity(safe_capacity),
            velocity_z: Vec::with_capacity(safe_capacity),

            acceleration_x: Vec::with_capacity(safe_capacity),
            acceleration_y: Vec::with_capacity(safe_capacity),
            acceleration_z: Vec::with_capacity(safe_capacity),

            color_r: Vec::with_capacity(safe_capacity),
            color_g: Vec::with_capacity(safe_capacity),
            color_b: Vec::with_capacity(safe_capacity),
            color_a: Vec::with_capacity(safe_capacity),

            size: Vec::with_capacity(safe_capacity),

            lifetime: Vec::with_capacity(safe_capacity),
            max_lifetime: Vec::with_capacity(safe_capacity),

            particle_type: Vec::with_capacity(safe_capacity),

            gravity_multiplier: Vec::with_capacity(safe_capacity),
            drag: Vec::with_capacity(safe_capacity),
            bounce: Vec::with_capacity(safe_capacity),

            rotation: Vec::with_capacity(safe_capacity),
            rotation_speed: Vec::with_capacity(safe_capacity),
            texture_frame: Vec::with_capacity(safe_capacity),
            animation_speed: Vec::with_capacity(safe_capacity),
            emissive: Vec::with_capacity(safe_capacity),
            emission_intensity: Vec::with_capacity(safe_capacity),

            size_curve_type: Vec::with_capacity(safe_capacity),
            size_curve_param1: Vec::with_capacity(safe_capacity),
            size_curve_param2: Vec::with_capacity(safe_capacity),
            size_curve_param3: Vec::with_capacity(safe_capacity),

            color_curve_type: Vec::with_capacity(safe_capacity),
            color_curve_param1: Vec::with_capacity(safe_capacity),
            color_curve_param2: Vec::with_capacity(safe_capacity),
    }
}

/// Clear all particle data
pub fn clear_particle_data(data: &mut ParticleData) {
    data.count = 0;

    data.position_x.clear();
    data.position_y.clear();
    data.position_z.clear();

    data.velocity_x.clear();
    data.velocity_y.clear();
    data.velocity_z.clear();

    data.acceleration_x.clear();
    data.acceleration_y.clear();
    data.acceleration_z.clear();

    data.color_r.clear();
    data.color_g.clear();
    data.color_b.clear();
    data.color_a.clear();

    data.size.clear();

    data.lifetime.clear();
    data.max_lifetime.clear();

    data.particle_type.clear();

    data.gravity_multiplier.clear();
    data.drag.clear();
    data.bounce.clear();

    data.rotation.clear();
    data.rotation_speed.clear();
    data.texture_frame.clear();
    data.animation_speed.clear();
    data.emissive.clear();
    data.emission_intensity.clear();

    data.size_curve_type.clear();
    data.size_curve_param1.clear();
    data.size_curve_param2.clear();
    data.size_curve_param3.clear();

    data.color_curve_type.clear();
    data.color_curve_param1.clear();
    data.color_curve_param2.clear();
}

/// Remove particle at index by swapping with last
pub fn remove_particle_swap(data: &mut ParticleData, index: usize) {
    if index >= data.count {
        return;
    }

    let last = data.count - 1;
    if index != last {
        data.position_x.swap(index, last);
        data.position_y.swap(index, last);
        data.position_z.swap(index, last);

        data.velocity_x.swap(index, last);
        data.velocity_y.swap(index, last);
        data.velocity_z.swap(index, last);

        data.acceleration_x.swap(index, last);
        data.acceleration_y.swap(index, last);
        data.acceleration_z.swap(index, last);

        data.color_r.swap(index, last);
        data.color_g.swap(index, last);
        data.color_b.swap(index, last);
        data.color_a.swap(index, last);

        data.size.swap(index, last);

        data.lifetime.swap(index, last);
        data.max_lifetime.swap(index, last);

        data.particle_type.swap(index, last);

        data.gravity_multiplier.swap(index, last);
        data.drag.swap(index, last);
        data.bounce.swap(index, last);

        data.rotation.swap(index, last);
        data.rotation_speed.swap(index, last);
        data.texture_frame.swap(index, last);
        data.animation_speed.swap(index, last);
        data.emissive.swap(index, last);
        data.emission_intensity.swap(index, last);

        data.size_curve_type.swap(index, last);
        data.size_curve_param1.swap(index, last);
        data.size_curve_param2.swap(index, last);
        data.size_curve_param3.swap(index, last);

        data.color_curve_type.swap(index, last);
        data.color_curve_param1.swap(index, last);
        data.color_curve_param2.swap(index, last);
    }

    // Remove last element
    data.position_x.pop();
    data.position_y.pop();
    data.position_z.pop();

    data.velocity_x.pop();
    data.velocity_y.pop();
    data.velocity_z.pop();

    data.acceleration_x.pop();
    data.acceleration_y.pop();
    data.acceleration_z.pop();

    data.color_r.pop();
    data.color_g.pop();
    data.color_b.pop();
    data.color_a.pop();

    data.size.pop();

    data.lifetime.pop();
    data.max_lifetime.pop();

    data.particle_type.pop();

    data.gravity_multiplier.pop();
    data.drag.pop();
    data.bounce.pop();

    data.rotation.pop();
    data.rotation_speed.pop();
    data.texture_frame.pop();
    data.animation_speed.pop();
    data.emissive.pop();
    data.emission_intensity.pop();

    data.size_curve_type.pop();
    data.size_curve_param1.pop();
    data.size_curve_param2.pop();
    data.size_curve_param3.pop();

    data.color_curve_type.pop();
    data.color_curve_param1.pop();
    data.color_curve_param2.pop();

    data.count -= 1;
}

/// Particle pool for efficient allocation
pub struct ParticlePool {
    /// Pre-allocated particle data
    pub data: ParticleData,
    /// Next available index
    pub next_free: usize,
}

/// Create a new particle pool
pub fn create_particle_pool(capacity: usize) -> ParticlePool {
    ParticlePool {
        data: create_particle_data(capacity),
        next_free: 0,
    }
}

/// Allocate space for new particles, returns start index and count allocated
pub fn allocate_particles(pool: &mut ParticlePool, count: usize) -> Option<(usize, usize)> {
    let available = pool.data.count.saturating_sub(pool.next_free);
    if available == 0 {
        return None;
    }

    let allocated = count.min(available);
    let start = pool.next_free;
    pool.next_free += allocated;

    Some((start, allocated))
}

/// Reset allocation pointer
pub fn reset_particle_pool(pool: &mut ParticlePool) {
    pool.next_free = 0;
}

/// Emitter data in SOA layout
pub struct EmitterData {
    /// Current number of active emitters
    pub count: usize,

    /// Emitter IDs
    pub id: Vec<u64>,

    /// Position
    pub position_x: Vec<f32>,
    pub position_y: Vec<f32>,
    pub position_z: Vec<f32>,

    /// Emission properties
    pub emission_rate: Vec<f32>,
    pub accumulated_particles: Vec<f32>,
    pub particle_type: Vec<u32>,

    /// Lifetime
    pub elapsed_time: Vec<f32>,
    pub duration: Vec<f32>, // negative means infinite

    /// Emission shape parameters
    pub shape_type: Vec<u8>, // 0=point, 1=sphere, 2=box, 3=cone
    pub shape_param1: Vec<f32>,
    pub shape_param2: Vec<f32>,
    pub shape_param3: Vec<f32>,

    /// Velocity parameters
    pub base_velocity_x: Vec<f32>,
    pub base_velocity_y: Vec<f32>,
    pub base_velocity_z: Vec<f32>,
    pub velocity_variance: Vec<f32>,
}

/// Create new emitter data buffer
pub fn create_emitter_data(capacity: usize) -> EmitterData {
        // Safety check: prevent excessive memory allocations
        let safe_capacity = if capacity > 100_000 {
            eprintln!(
                "WARNING: EmitterData capacity {} is extremely large, clamping to 10K",
                capacity
            );
            10_000
        } else {
            capacity
        };
        EmitterData {
            count: 0,

            id: Vec::with_capacity(safe_capacity),

            position_x: Vec::with_capacity(safe_capacity),
            position_y: Vec::with_capacity(safe_capacity),
            position_z: Vec::with_capacity(safe_capacity),

            emission_rate: Vec::with_capacity(safe_capacity),
            accumulated_particles: Vec::with_capacity(safe_capacity),
            particle_type: Vec::with_capacity(safe_capacity),

            elapsed_time: Vec::with_capacity(safe_capacity),
            duration: Vec::with_capacity(safe_capacity),

            shape_type: Vec::with_capacity(safe_capacity),
            shape_param1: Vec::with_capacity(safe_capacity),
            shape_param2: Vec::with_capacity(safe_capacity),
            shape_param3: Vec::with_capacity(safe_capacity),

            base_velocity_x: Vec::with_capacity(safe_capacity),
            base_velocity_y: Vec::with_capacity(safe_capacity),
            base_velocity_z: Vec::with_capacity(safe_capacity),
            velocity_variance: Vec::with_capacity(safe_capacity),
        }
}

/// Clear all emitter data
pub fn clear_emitter_data(data: &mut EmitterData) {
    data.count = 0;

    data.id.clear();

    data.position_x.clear();
    data.position_y.clear();
    data.position_z.clear();

    data.emission_rate.clear();
    data.accumulated_particles.clear();
    data.particle_type.clear();

    data.elapsed_time.clear();
    data.duration.clear();

    data.shape_type.clear();
    data.shape_param1.clear();
    data.shape_param2.clear();
    data.shape_param3.clear();

    data.base_velocity_x.clear();
    data.base_velocity_y.clear();
    data.base_velocity_z.clear();
    data.velocity_variance.clear();
}

/// Render data for GPU
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleGPUData {
    pub position: [f32; 3],
    pub size: f32,
    pub color: [f32; 4],
    pub rotation: f32,
    pub texture_index: u32,
    pub _padding: [f32; 2],
}

/// Convert particle data to GPU format
pub fn prepare_render_data(particles: &ParticleData, gpu_buffer: &mut Vec<ParticleGPUData>) {
    gpu_buffer.clear();
    gpu_buffer.reserve(particles.count);

    for i in 0..particles.count {
        gpu_buffer.push(ParticleGPUData {
            position: [
                particles.position_x[i],
                particles.position_y[i],
                particles.position_z[i],
            ],
            size: particles.size[i],
            color: [
                particles.color_r[i],
                particles.color_g[i],
                particles.color_b[i],
                particles.color_a[i],
            ],
            rotation: particles.rotation[i],
            texture_index: particles.texture_frame[i],
            _padding: [0.0, 0.0],
        });
    }
}
