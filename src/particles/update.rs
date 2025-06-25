use glam::Vec3;
use rand::{thread_rng, Rng};

use crate::particles::particle_data::{EmitterData, ParticleData, remove_particle_swap};
use crate::{BlockId, VoxelPos, World};

/// Update all particles in the system
pub fn update_particles(
    particles: &mut ParticleData,
    world: &World,
    dt: f32,
    wind_velocity: Vec3,
    collision_enabled: bool,
) {
    // Reset accelerations
    for i in 0..particles.count {
        particles.acceleration_x[i] = 0.0;
        particles.acceleration_y[i] = 0.0;
        particles.acceleration_z[i] = 0.0;
    }

    // Apply physics
    apply_gravity(particles, dt);
    apply_drag(particles, dt);
    apply_wind(particles, wind_velocity, dt);

    // Update velocities and positions
    integrate_motion(particles, dt);

    // Handle collisions if enabled
    if collision_enabled {
        handle_collisions(particles, world);
    }

    // Update visual properties
    update_lifetime(particles, dt);
    update_rotation(particles, dt);
    update_animation(particles, dt);
    update_sizes(particles);
    update_colors(particles);

    // Remove dead particles
    remove_dead_particles(particles);
}

/// Apply gravity to all particles
pub fn apply_gravity(particles: &mut ParticleData, dt: f32) {
    // Use voxel-scaled gravity from constants (-98.1 voxels/s²)
    // Note: We negate because constants::physics_constants::GRAVITY is already negative
    let gravity = -crate::constants::physics_constants::GRAVITY;

    for i in 0..particles.count {
        particles.acceleration_y[i] -= gravity * particles.gravity_multiplier[i];
    }
}

/// Apply drag forces
pub fn apply_drag(particles: &mut ParticleData, dt: f32) {
    for i in 0..particles.count {
        let drag = particles.drag[i];
        if drag > 0.0 {
            let factor = 1.0 - drag * dt;
            particles.velocity_x[i] *= factor;
            particles.velocity_y[i] *= factor;
            particles.velocity_z[i] *= factor;
        }
    }
}

/// Apply wind forces
pub fn apply_wind(particles: &mut ParticleData, wind: Vec3, dt: f32) {
    if wind.length_squared() == 0.0 {
        return;
    }

    for i in 0..particles.count {
        // Wind affects particles based on their drag coefficient
        let wind_effect = particles.drag[i] * 0.5;
        particles.acceleration_x[i] += wind.x * wind_effect;
        particles.acceleration_y[i] += wind.y * wind_effect;
        particles.acceleration_z[i] += wind.z * wind_effect;
    }
}

/// Integrate motion (velocity and position)
pub fn integrate_motion(particles: &mut ParticleData, dt: f32) {
    for i in 0..particles.count {
        // Update velocity
        particles.velocity_x[i] += particles.acceleration_x[i] * dt;
        particles.velocity_y[i] += particles.acceleration_y[i] * dt;
        particles.velocity_z[i] += particles.acceleration_z[i] * dt;

        // Update position
        particles.position_x[i] += particles.velocity_x[i] * dt;
        particles.position_y[i] += particles.velocity_y[i] * dt;
        particles.position_z[i] += particles.velocity_z[i] * dt;
    }
}

/// Check if a block ID represents a solid block
fn is_block_solid(block_id: BlockId) -> bool {
    // Air, water, lava, and various vegetation are not solid
    match block_id {
        BlockId::AIR
        | BlockId::WATER
        | BlockId::LAVA
        | BlockId::TALL_GRASS
        | BlockId::FLOWER_RED
        | BlockId::FLOWER_YELLOW
        | BlockId::MUSHROOM_RED
        | BlockId::MUSHROOM_BROWN
        | BlockId::DEAD_BUSH
        | BlockId::SUGAR_CANE
        | BlockId::VINES => false,
        _ => true,
    }
}

/// Handle particle collisions with world
pub fn handle_collisions(particles: &mut ParticleData, world: &World) {
    for i in 0..particles.count {
        let pos = Vec3::new(
            particles.position_x[i],
            particles.position_y[i],
            particles.position_z[i],
        );

        // Convert to voxel position
        let voxel_pos = VoxelPos::new(
            pos.x.floor() as i32,
            pos.y.floor() as i32,
            pos.z.floor() as i32,
        );

        // Get block at position
        let block_id = world.get_block(voxel_pos);

        // Check if particle is inside a solid block
        if is_block_solid(block_id) {
            // Simple bounce back
            let bounce = particles.bounce[i];

            // Reflect velocity
            particles.velocity_x[i] *= -bounce;
            particles.velocity_y[i] *= -bounce;
            particles.velocity_z[i] *= -bounce;

            // Move particle out of solid
            particles.position_x[i] -= particles.velocity_x[i] * 0.016;
            particles.position_y[i] -= particles.velocity_y[i] * 0.016;
            particles.position_z[i] -= particles.velocity_z[i] * 0.016;
        }
    }
}

/// Update particle lifetimes
pub fn update_lifetime(particles: &mut ParticleData, dt: f32) {
    for i in 0..particles.count {
        particles.lifetime[i] -= dt;
    }
}

/// Update particle rotation
pub fn update_rotation(particles: &mut ParticleData, dt: f32) {
    for i in 0..particles.count {
        particles.rotation[i] += particles.rotation_speed[i] * dt;
    }
}

/// Update animation frames
pub fn update_animation(particles: &mut ParticleData, dt: f32) {
    for i in 0..particles.count {
        if particles.animation_speed[i] > 0.0 {
            let elapsed = particles.max_lifetime[i] - particles.lifetime[i];
            particles.texture_frame[i] = (elapsed * particles.animation_speed[i]) as u32;
        }
    }
}

/// Update particle sizes based on curves
pub fn update_sizes(particles: &mut ParticleData) {
    for i in 0..particles.count {
        let t = 1.0 - (particles.lifetime[i] / particles.max_lifetime[i]);

        particles.size[i] = match particles.size_curve_type[i] {
            0 => particles.size[i], // Constant
            1 => {
                // Linear
                let start = particles.size_curve_param1[i];
                let end = particles.size_curve_param2[i];
                start + (end - start) * t
            }
            2 => {
                // Grow-shrink
                let start = particles.size_curve_param1[i];
                let peak = particles.size_curve_param2[i];
                let end = particles.size_curve_param3[i];
                if t < 0.5 {
                    start + (peak - start) * (t * 2.0)
                } else {
                    peak + (end - peak) * ((t - 0.5) * 2.0)
                }
            }
            _ => particles.size[i],
        };
    }
}

/// Update particle colors based on curves
pub fn update_colors(particles: &mut ParticleData) {
    for i in 0..particles.count {
        let t = 1.0 - (particles.lifetime[i] / particles.max_lifetime[i]);

        match particles.color_curve_type[i] {
            0 => {} // Constant, no update needed
            1 => {
                // Fade out
                particles.color_a[i] = 1.0 - t;
            }
            2 => {
                // Linear interpolation (params store end color in rgb channels)
                let start_a = particles.color_a[i];
                particles.color_r[i] = particles.color_r[i]
                    + (particles.color_curve_param1[i] - particles.color_r[i]) * t;
                particles.color_g[i] = particles.color_g[i]
                    + (particles.color_curve_param2[i] - particles.color_g[i]) * t;
                particles.color_b[i] = particles.color_b[i]
                    + (particles.color_curve_param1[i] - particles.color_b[i]) * t;
                particles.color_a[i] = start_a * (1.0 - t);
            }
            3 => {
                // Temperature
                let temp = particles.color_curve_param1[i]
                    + (particles.color_curve_param2[i] - particles.color_curve_param1[i]) * t;
                let color = temperature_to_color(temp);
                particles.color_r[i] = color.0;
                particles.color_g[i] = color.1;
                particles.color_b[i] = color.2;
            }
            _ => {}
        }
    }
}

/// Remove dead particles
pub fn remove_dead_particles(particles: &mut ParticleData) {
    let mut i = 0;
    while i < particles.count {
        if particles.lifetime[i] <= 0.0 {
            remove_particle_swap(particles, i);
        } else {
            i += 1;
        }
    }
}

/// Convert temperature to color
fn temperature_to_color(temp: f32) -> (f32, f32, f32) {
    let t = temp.clamp(0.0, 1.0);

    if t < 0.5 {
        // Black to red to orange
        let t2 = t * 2.0;
        (t2, t2 * 0.3, 0.0)
    } else {
        // Orange to yellow to white
        let t2 = (t - 0.5) * 2.0;
        (1.0, 0.3 + t2 * 0.7, t2)
    }
}

/// Update emitters and spawn new particles
pub fn update_emitters(
    emitters: &mut EmitterData,
    particles: &mut ParticleData,
    dt: f32,
    next_id: &mut u64,
) -> usize {
    let mut total_spawned = 0;
    let mut rng = thread_rng();

    // Update each emitter
    let mut i = 0;
    while i < emitters.count {
        // Update elapsed time
        emitters.elapsed_time[i] += dt;

        // Check if emitter should be removed
        if emitters.duration[i] >= 0.0 && emitters.elapsed_time[i] >= emitters.duration[i] {
            // Remove finished emitter
            remove_emitter_at(emitters, i);
            continue;
        }

        // Calculate particles to spawn
        emitters.accumulated_particles[i] += emitters.emission_rate[i] * dt;
        let to_spawn = emitters.accumulated_particles[i] as usize;
        emitters.accumulated_particles[i] -= to_spawn as f32;

        // Spawn particles
        for _ in 0..to_spawn {
            if particles.count >= particles.position_x.capacity() {
                break;
            }

            // Generate spawn position based on shape
            let spawn_pos = generate_spawn_position(emitters, i, &mut rng);

            // Generate velocity
            let base_vel = Vec3::new(
                emitters.base_velocity_x[i],
                emitters.base_velocity_y[i],
                emitters.base_velocity_z[i],
            );
            let variance = emitters.velocity_variance[i];
            let velocity = base_vel
                + Vec3::new(
                    rng.gen_range(-variance..variance),
                    rng.gen_range(-variance..variance),
                    rng.gen_range(-variance..variance),
                );

            // Add particle
            spawn_particle(particles, spawn_pos, velocity, emitters.particle_type[i]);

            total_spawned += 1;
        }

        i += 1;
    }

    total_spawned
}

/// Generate spawn position based on emitter shape
fn generate_spawn_position(emitters: &EmitterData, index: usize, rng: &mut impl Rng) -> Vec3 {
    let base_pos = Vec3::new(
        emitters.position_x[index],
        emitters.position_y[index],
        emitters.position_z[index],
    );

    match emitters.shape_type[index] {
        0 => base_pos, // Point
        1 => {
            // Sphere
            let radius = emitters.shape_param1[index];
            let theta = rng.gen_range(0.0..std::f32::consts::TAU);
            let phi = rng.gen_range(0.0..std::f32::consts::PI);
            let r = rng.gen_range(0.0..radius);

            base_pos
                + Vec3::new(
                    r * phi.sin() * theta.cos(),
                    r * phi.cos(),
                    r * phi.sin() * theta.sin(),
                )
        }
        2 => {
            // Box
            let half_x = emitters.shape_param1[index] * 0.5;
            let half_y = emitters.shape_param2[index] * 0.5;
            let half_z = emitters.shape_param3[index] * 0.5;

            base_pos
                + Vec3::new(
                    rng.gen_range(-half_x..half_x),
                    rng.gen_range(-half_y..half_y),
                    rng.gen_range(-half_z..half_z),
                )
        }
        3 => {
            // Cone
            let angle = emitters.shape_param1[index];
            let height = emitters.shape_param2[index];

            let h = rng.gen_range(0.0..height);
            let r = h * angle.tan();
            let theta = rng.gen_range(0.0..std::f32::consts::TAU);

            base_pos + Vec3::new(r * theta.cos(), -h, r * theta.sin())
        }
        _ => base_pos,
    }
}

/// Spawn a single particle
pub fn spawn_particle(
    particles: &mut ParticleData,
    position: Vec3,
    velocity: Vec3,
    particle_type: u32,
) {
    if particles.count >= particles.position_x.capacity() {
        return;
    }

    // Get default properties for particle type
    let (color, size, lifetime) = get_particle_defaults(particle_type);
    let properties = get_particle_properties(particle_type);

    // Add to buffers
    particles.position_x.push(position.x);
    particles.position_y.push(position.y);
    particles.position_z.push(position.z);

    particles.velocity_x.push(velocity.x);
    particles.velocity_y.push(velocity.y);
    particles.velocity_z.push(velocity.z);

    particles.acceleration_x.push(0.0);
    particles.acceleration_y.push(0.0);
    particles.acceleration_z.push(0.0);

    particles.color_r.push(color.x);
    particles.color_g.push(color.y);
    particles.color_b.push(color.z);
    particles.color_a.push(color.w);

    particles.size.push(size);

    particles.lifetime.push(lifetime);
    particles.max_lifetime.push(lifetime);

    particles.particle_type.push(particle_type);

    particles.gravity_multiplier.push(properties.gravity);
    particles.drag.push(properties.drag);
    particles.bounce.push(properties.bounce);

    particles.rotation.push(properties.rotation);
    particles.rotation_speed.push(properties.rotation_speed);
    particles.texture_frame.push(properties.texture_frame);
    particles.animation_speed.push(properties.animation_speed);
    particles.emissive.push(properties.emissive);
    particles
        .emission_intensity
        .push(properties.emission_intensity);

    particles.size_curve_type.push(properties.size_curve_type);
    particles
        .size_curve_param1
        .push(properties.size_curve_param1);
    particles
        .size_curve_param2
        .push(properties.size_curve_param2);
    particles
        .size_curve_param3
        .push(properties.size_curve_param3);

    particles.color_curve_type.push(properties.color_curve_type);
    particles
        .color_curve_param1
        .push(properties.color_curve_param1);
    particles
        .color_curve_param2
        .push(properties.color_curve_param2);

    particles.count += 1;
}

/// Get default properties for particle type
fn get_particle_defaults(particle_type: u32) -> (glam::Vec4, f32, f32) {
    match particle_type {
        0 => (glam::Vec4::new(0.6, 0.6, 0.8, 0.6), 0.05, 2.0), // Rain
        1 => (glam::Vec4::new(1.0, 1.0, 1.0, 0.8), 0.1, 5.0),  // Snow
        2 => (glam::Vec4::new(0.3, 0.3, 0.3, 0.5), 0.5, 3.0),  // Smoke
        3 => (glam::Vec4::new(1.0, 0.5, 0.1, 1.0), 0.3, 1.0),  // Fire
        4 => (glam::Vec4::new(1.0, 0.8, 0.2, 1.0), 0.05, 0.5), // Spark
        5 => (glam::Vec4::new(0.7, 0.6, 0.5, 0.4), 0.2, 2.0),  // Dust
        _ => (glam::Vec4::new(1.0, 1.0, 1.0, 1.0), 0.1, 1.0),
    }
}

struct ParticleDefaults {
    gravity: f32,
    drag: f32,
    bounce: f32,
    rotation: f32,
    rotation_speed: f32,
    texture_frame: u32,
    animation_speed: f32,
    emissive: bool,
    emission_intensity: f32,
    size_curve_type: u8,
    size_curve_param1: f32,
    size_curve_param2: f32,
    size_curve_param3: f32,
    color_curve_type: u8,
    color_curve_param1: f32,
    color_curve_param2: f32,
}

/// Get particle properties for type
fn get_particle_properties(particle_type: u32) -> ParticleDefaults {
    match particle_type {
        0 => ParticleDefaults {
            // Rain
            gravity: 2.0,
            drag: 0.1,
            bounce: 0.0,
            rotation: 0.0,
            rotation_speed: 0.0,
            texture_frame: 0,
            animation_speed: 0.0,
            emissive: false,
            emission_intensity: 0.0,
            size_curve_type: 0, // Constant
            size_curve_param1: 0.0,
            size_curve_param2: 0.0,
            size_curve_param3: 0.0,
            color_curve_type: 0, // Constant
            color_curve_param1: 0.0,
            color_curve_param2: 0.0,
        },
        1 => ParticleDefaults {
            // Snow
            gravity: 0.1,
            drag: 0.5,
            bounce: 0.0,
            rotation: 0.0,
            rotation_speed: 1.0,
            texture_frame: 0,
            animation_speed: 0.0,
            emissive: false,
            emission_intensity: 0.0,
            size_curve_type: 0,
            size_curve_param1: 0.0,
            size_curve_param2: 0.0,
            size_curve_param3: 0.0,
            color_curve_type: 0,
            color_curve_param1: 0.0,
            color_curve_param2: 0.0,
        },
        3 => ParticleDefaults {
            // Fire
            gravity: -0.5,
            drag: 0.8,
            bounce: 0.0,
            rotation: 0.0,
            rotation_speed: 0.0,
            texture_frame: 0,
            animation_speed: 10.0,
            emissive: true,
            emission_intensity: 1.0,
            size_curve_type: 1, // Linear
            size_curve_param1: 0.3,
            size_curve_param2: 0.0,
            size_curve_param3: 0.0,
            color_curve_type: 3, // Temperature
            color_curve_param1: 1.0,
            color_curve_param2: 0.0,
        },
        2 => ParticleDefaults {
            // Smoke
            gravity: -0.2,
            drag: 0.5,
            bounce: 0.0,
            rotation: 0.0,
            rotation_speed: 0.5,
            texture_frame: 0,
            animation_speed: 0.0,
            emissive: false,
            emission_intensity: 0.0,
            size_curve_type: 1, // Linear
            size_curve_param1: 0.3,
            size_curve_param2: 1.0,
            size_curve_param3: 0.0,
            color_curve_type: 1, // Fade out
            color_curve_param1: 0.0,
            color_curve_param2: 0.0,
        },
        _ => ParticleDefaults {
            // Default
            gravity: 1.0,
            drag: 0.0,
            bounce: 0.5,
            rotation: 0.0,
            rotation_speed: 0.0,
            texture_frame: 0,
            animation_speed: 0.0,
            emissive: false,
            emission_intensity: 0.0,
            size_curve_type: 0,
            size_curve_param1: 0.0,
            size_curve_param2: 0.0,
            size_curve_param3: 0.0,
            color_curve_type: 0,
            color_curve_param1: 0.0,
            color_curve_param2: 0.0,
        },
    }
}

/// Remove emitter at index
fn remove_emitter_at(emitters: &mut EmitterData, index: usize) {
    if index >= emitters.count {
        return;
    }

    let last = emitters.count - 1;
    if index != last {
        emitters.id.swap(index, last);

        emitters.position_x.swap(index, last);
        emitters.position_y.swap(index, last);
        emitters.position_z.swap(index, last);

        emitters.emission_rate.swap(index, last);
        emitters.accumulated_particles.swap(index, last);
        emitters.particle_type.swap(index, last);

        emitters.elapsed_time.swap(index, last);
        emitters.duration.swap(index, last);

        emitters.shape_type.swap(index, last);
        emitters.shape_param1.swap(index, last);
        emitters.shape_param2.swap(index, last);
        emitters.shape_param3.swap(index, last);

        emitters.base_velocity_x.swap(index, last);
        emitters.base_velocity_y.swap(index, last);
        emitters.base_velocity_z.swap(index, last);
        emitters.velocity_variance.swap(index, last);
    }

    // Remove last element
    emitters.id.pop();

    emitters.position_x.pop();
    emitters.position_y.pop();
    emitters.position_z.pop();

    emitters.emission_rate.pop();
    emitters.accumulated_particles.pop();
    emitters.particle_type.pop();

    emitters.elapsed_time.pop();
    emitters.duration.pop();

    emitters.shape_type.pop();
    emitters.shape_param1.pop();
    emitters.shape_param2.pop();
    emitters.shape_param3.pop();

    emitters.base_velocity_x.pop();
    emitters.base_velocity_y.pop();
    emitters.base_velocity_z.pop();
    emitters.velocity_variance.pop();

    emitters.count -= 1;
}

/// Apply force field to particles
pub fn apply_force_field(particles: &mut ParticleData, center: Vec3, strength: f32, radius: f32) {
    let radius_sq = radius * radius;

    for i in 0..particles.count {
        let pos = Vec3::new(
            particles.position_x[i],
            particles.position_y[i],
            particles.position_z[i],
        );

        let delta = center - pos;
        let dist_sq = delta.length_squared();

        if dist_sq < radius_sq && dist_sq > 0.01 {
            let dist = dist_sq.sqrt();
            let force = (strength / dist_sq) * (1.0 - dist / radius);
            let force_dir = delta / dist;

            particles.acceleration_x[i] += force_dir.x * force;
            particles.acceleration_y[i] += force_dir.y * force;
            particles.acceleration_z[i] += force_dir.z * force;
        }
    }
}

/// Apply vortex force to particles
pub fn apply_vortex(
    particles: &mut ParticleData,
    center: Vec3,
    axis: Vec3,
    strength: f32,
    radius: f32,
) {
    let axis_norm = axis.normalize();
    let radius_sq = radius * radius;

    for i in 0..particles.count {
        let pos = Vec3::new(
            particles.position_x[i],
            particles.position_y[i],
            particles.position_z[i],
        );

        let to_center = center - pos;
        let dist_sq = to_center.length_squared();

        if dist_sq < radius_sq && dist_sq > 0.01 {
            let dist = dist_sq.sqrt();
            let tangent = axis_norm.cross(to_center).normalize();
            let force = strength * (1.0 - dist / radius);

            particles.acceleration_x[i] += tangent.x * force;
            particles.acceleration_y[i] += tangent.y * force;
            particles.acceleration_z[i] += tangent.z * force;
        }
    }
}

/// Apply turbulence using noise
pub fn apply_turbulence(particles: &mut ParticleData, strength: f32, scale: f32, time: f32) {
    for i in 0..particles.count {
        let pos = Vec3::new(
            particles.position_x[i],
            particles.position_y[i],
            particles.position_z[i],
        );

        // Simple pseudo-random turbulence (in real implementation, use proper noise)
        let noise_x = ((pos.x * scale + time).sin() + (pos.z * scale * 1.3).cos()) * 0.5;
        let noise_y = ((pos.y * scale + time * 1.1).sin() + (pos.x * scale * 0.7).cos()) * 0.5;
        let noise_z = ((pos.z * scale + time * 0.9).sin() + (pos.y * scale * 1.5).cos()) * 0.5;

        particles.acceleration_x[i] += noise_x * strength;
        particles.acceleration_y[i] += noise_y * strength;
        particles.acceleration_z[i] += noise_z * strength;
    }
}
