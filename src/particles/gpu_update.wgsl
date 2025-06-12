// GPU compute shader for particle updates
// This demonstrates how the data-oriented layout enables efficient GPU processing

struct ParticleData {
    position: vec3<f32>,
    size: f32,
    velocity: vec3<f32>,
    lifetime: f32,
    acceleration: vec3<f32>,
    max_lifetime: f32,
    color: vec4<f32>,
    gravity_multiplier: f32,
    drag: f32,
    bounce: f32,
    rotation: f32,
    rotation_speed: f32,
    particle_type: u32,
    texture_frame: u32,
    size_curve_type: u32,
    color_curve_type: u32,
}

struct SimParams {
    dt: f32,
    time: f32,
    wind_velocity: vec3<f32>,
    gravity: f32,
    particle_count: u32,
}

@group(0) @binding(0) var<storage, read_write> particles: array<ParticleData>;
@group(0) @binding(1) var<uniform> params: SimParams;

// Temperature to color conversion
fn temperature_to_color(temp: f32) -> vec3<f32> {
    let t = clamp(temp, 0.0, 1.0);
    
    if (t < 0.5) {
        // Black to red to orange
        let t2 = t * 2.0;
        return vec3<f32>(t2, t2 * 0.3, 0.0);
    } else {
        // Orange to yellow to white
        let t2 = (t - 0.5) * 2.0;
        return vec3<f32>(1.0, 0.3 + t2 * 0.7, t2);
    }
}

// Update particle size based on curve
fn update_size(particle: ptr<storage, ParticleData, read_write>) {
    let t = 1.0 - ((*particle).lifetime / (*particle).max_lifetime);
    
    switch ((*particle).size_curve_type) {
        case 0u: { // Constant
            // Size stays the same
        }
        case 1u: { // Linear
            let start = (*particle).size;
            let end = 0.0; // Would be stored in additional params
            (*particle).size = mix(start, end, t);
        }
        case 2u: { // Grow-shrink
            if (t < 0.5) {
                (*particle).size = mix((*particle).size, (*particle).size * 1.5, t * 2.0);
            } else {
                (*particle).size = mix((*particle).size * 1.5, 0.0, (t - 0.5) * 2.0);
            }
        }
        default: {}
    }
}

// Update particle color based on curve
fn update_color(particle: ptr<storage, ParticleData, read_write>) {
    let t = 1.0 - ((*particle).lifetime / (*particle).max_lifetime);
    
    switch ((*particle).color_curve_type) {
        case 0u: { // Constant
            // Color stays the same
        }
        case 1u: { // Fade out
            (*particle).color.a = 1.0 - t;
        }
        case 3u: { // Temperature
            let temp = mix(1.0, 0.0, t); // Would use stored params
            (*particle).color.rgb = temperature_to_color(temp);
        }
        default: {}
    }
}

@compute @workgroup_size(64)
fn update_particles(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= params.particle_count) {
        return;
    }
    
    let particle = &particles[idx];
    
    // Skip dead particles
    if ((*particle).lifetime <= 0.0) {
        return;
    }
    
    // Reset acceleration
    (*particle).acceleration = vec3<f32>(0.0, 0.0, 0.0);
    
    // Apply gravity
    (*particle).acceleration.y -= params.gravity * (*particle).gravity_multiplier;
    
    // Apply wind (affected by drag)
    let wind_effect = params.wind_velocity * (*particle).drag * 0.5;
    (*particle).acceleration += wind_effect;
    
    // Apply drag to velocity
    let drag_factor = 1.0 - (*particle).drag * params.dt;
    (*particle).velocity *= drag_factor;
    
    // Integrate motion
    (*particle).velocity += (*particle).acceleration * params.dt;
    (*particle).position += (*particle).velocity * params.dt;
    
    // Update lifetime
    (*particle).lifetime -= params.dt;
    
    // Update rotation
    (*particle).rotation += (*particle).rotation_speed * params.dt;
    
    // Update animation frame
    if ((*particle).texture_frame > 0u) {
        let elapsed = (*particle).max_lifetime - (*particle).lifetime;
        (*particle).texture_frame = u32(elapsed * 10.0); // Animation speed hardcoded for demo
    }
    
    // Update visual properties
    update_size(particle);
    update_color(particle);
}

// Separate kernel for applying forces (can be called multiple times per frame)
@compute @workgroup_size(64)
fn apply_force_field(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
) {
    let idx = global_id.x;
    if (idx >= params.particle_count) {
        return;
    }
    
    let particle = &particles[idx];
    
    // Example: Apply vortex force around origin
    let center = vec3<f32>(0.0, 10.0, 0.0);
    let to_center = center - (*particle).position;
    let dist = length(to_center);
    
    if (dist > 0.1 && dist < 20.0) {
        let axis = vec3<f32>(0.0, 1.0, 0.0);
        let tangent = normalize(cross(axis, to_center));
        let force = 10.0 * (1.0 - dist / 20.0);
        
        (*particle).acceleration += tangent * force;
    }
}

// Kernel for spawning particles from emitters
struct EmitterData {
    position: vec3<f32>,
    emission_rate: f32,
    base_velocity: vec3<f32>,
    velocity_variance: f32,
    particle_type: u32,
    shape_type: u32,
    shape_param1: f32,
    shape_param2: f32,
}

@group(1) @binding(0) var<storage, read> emitters: array<EmitterData>;
@group(1) @binding(1) var<storage, read_write> spawn_queue: array<ParticleData>;
@group(1) @binding(2) var<storage, read_write> spawn_count: atomic<u32>;

@compute @workgroup_size(32)
fn spawn_from_emitters(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let emitter_idx = global_id.x;
    let emitter = emitters[emitter_idx];
    
    // Calculate particles to spawn this frame
    let to_spawn = u32(emitter.emission_rate * params.dt);
    
    // Use thread ID as seed for pseudo-random
    var seed = global_id.x * 1664525u + 1013904223u;
    
    for (var i = 0u; i < to_spawn; i++) {
        // Get spawn index atomically
        let spawn_idx = atomicAdd(&spawn_count, 1u);
        if (spawn_idx >= arrayLength(&spawn_queue)) {
            break;
        }
        
        // Generate random spawn position based on shape
        var spawn_pos = emitter.position;
        
        // Simple random number generation
        seed = seed * 1664525u + 1013904223u;
        let rand1 = f32(seed) / 4294967296.0;
        seed = seed * 1664525u + 1013904223u;
        let rand2 = f32(seed) / 4294967296.0;
        seed = seed * 1664525u + 1013904223u;
        let rand3 = f32(seed) / 4294967296.0;
        
        if (emitter.shape_type == 1u) { // Sphere
            let theta = rand1 * 6.28318530718;
            let phi = rand2 * 3.14159265359;
            let r = rand3 * emitter.shape_param1;
            
            spawn_pos += vec3<f32>(
                r * sin(phi) * cos(theta),
                r * cos(phi),
                r * sin(phi) * sin(theta)
            );
        }
        
        // Generate velocity
        let velocity = emitter.base_velocity + vec3<f32>(
            (rand1 - 0.5) * 2.0 * emitter.velocity_variance,
            (rand2 - 0.5) * 2.0 * emitter.velocity_variance,
            (rand3 - 0.5) * 2.0 * emitter.velocity_variance
        );
        
        // Initialize particle
        spawn_queue[spawn_idx] = ParticleData(
            spawn_pos,
            0.1, // size
            velocity,
            1.0, // lifetime
            vec3<f32>(0.0, 0.0, 0.0), // acceleration
            1.0, // max_lifetime
            vec4<f32>(1.0, 1.0, 1.0, 1.0), // color
            1.0, // gravity_multiplier
            0.1, // drag
            0.5, // bounce
            0.0, // rotation
            0.0, // rotation_speed
            emitter.particle_type,
            0u, // texture_frame
            0u, // size_curve_type
            0u  // color_curve_type
        );
    }
}