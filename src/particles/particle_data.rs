
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

impl ParticleData {
    /// Create a new particle data buffer with pre-allocated capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            count: 0,
            
            position_x: Vec::with_capacity(capacity),
            position_y: Vec::with_capacity(capacity),
            position_z: Vec::with_capacity(capacity),
            
            velocity_x: Vec::with_capacity(capacity),
            velocity_y: Vec::with_capacity(capacity),
            velocity_z: Vec::with_capacity(capacity),
            
            acceleration_x: Vec::with_capacity(capacity),
            acceleration_y: Vec::with_capacity(capacity),
            acceleration_z: Vec::with_capacity(capacity),
            
            color_r: Vec::with_capacity(capacity),
            color_g: Vec::with_capacity(capacity),
            color_b: Vec::with_capacity(capacity),
            color_a: Vec::with_capacity(capacity),
            
            size: Vec::with_capacity(capacity),
            
            lifetime: Vec::with_capacity(capacity),
            max_lifetime: Vec::with_capacity(capacity),
            
            particle_type: Vec::with_capacity(capacity),
            
            gravity_multiplier: Vec::with_capacity(capacity),
            drag: Vec::with_capacity(capacity),
            bounce: Vec::with_capacity(capacity),
            
            rotation: Vec::with_capacity(capacity),
            rotation_speed: Vec::with_capacity(capacity),
            texture_frame: Vec::with_capacity(capacity),
            animation_speed: Vec::with_capacity(capacity),
            emissive: Vec::with_capacity(capacity),
            emission_intensity: Vec::with_capacity(capacity),
            
            size_curve_type: Vec::with_capacity(capacity),
            size_curve_param1: Vec::with_capacity(capacity),
            size_curve_param2: Vec::with_capacity(capacity),
            size_curve_param3: Vec::with_capacity(capacity),
            
            color_curve_type: Vec::with_capacity(capacity),
            color_curve_param1: Vec::with_capacity(capacity),
            color_curve_param2: Vec::with_capacity(capacity),
        }
    }
    
    /// Clear all particle data
    pub fn clear(&mut self) {
        self.count = 0;
        
        self.position_x.clear();
        self.position_y.clear();
        self.position_z.clear();
        
        self.velocity_x.clear();
        self.velocity_y.clear();
        self.velocity_z.clear();
        
        self.acceleration_x.clear();
        self.acceleration_y.clear();
        self.acceleration_z.clear();
        
        self.color_r.clear();
        self.color_g.clear();
        self.color_b.clear();
        self.color_a.clear();
        
        self.size.clear();
        
        self.lifetime.clear();
        self.max_lifetime.clear();
        
        self.particle_type.clear();
        
        self.gravity_multiplier.clear();
        self.drag.clear();
        self.bounce.clear();
        
        self.rotation.clear();
        self.rotation_speed.clear();
        self.texture_frame.clear();
        self.animation_speed.clear();
        self.emissive.clear();
        self.emission_intensity.clear();
        
        self.size_curve_type.clear();
        self.size_curve_param1.clear();
        self.size_curve_param2.clear();
        self.size_curve_param3.clear();
        
        self.color_curve_type.clear();
        self.color_curve_param1.clear();
        self.color_curve_param2.clear();
    }
    
    /// Remove particle at index by swapping with last
    pub fn remove_swap(&mut self, index: usize) {
        if index >= self.count {
            return;
        }
        
        let last = self.count - 1;
        if index != last {
            self.position_x.swap(index, last);
            self.position_y.swap(index, last);
            self.position_z.swap(index, last);
            
            self.velocity_x.swap(index, last);
            self.velocity_y.swap(index, last);
            self.velocity_z.swap(index, last);
            
            self.acceleration_x.swap(index, last);
            self.acceleration_y.swap(index, last);
            self.acceleration_z.swap(index, last);
            
            self.color_r.swap(index, last);
            self.color_g.swap(index, last);
            self.color_b.swap(index, last);
            self.color_a.swap(index, last);
            
            self.size.swap(index, last);
            
            self.lifetime.swap(index, last);
            self.max_lifetime.swap(index, last);
            
            self.particle_type.swap(index, last);
            
            self.gravity_multiplier.swap(index, last);
            self.drag.swap(index, last);
            self.bounce.swap(index, last);
            
            self.rotation.swap(index, last);
            self.rotation_speed.swap(index, last);
            self.texture_frame.swap(index, last);
            self.animation_speed.swap(index, last);
            self.emissive.swap(index, last);
            self.emission_intensity.swap(index, last);
            
            self.size_curve_type.swap(index, last);
            self.size_curve_param1.swap(index, last);
            self.size_curve_param2.swap(index, last);
            self.size_curve_param3.swap(index, last);
            
            self.color_curve_type.swap(index, last);
            self.color_curve_param1.swap(index, last);
            self.color_curve_param2.swap(index, last);
        }
        
        // Remove last element
        self.position_x.pop();
        self.position_y.pop();
        self.position_z.pop();
        
        self.velocity_x.pop();
        self.velocity_y.pop();
        self.velocity_z.pop();
        
        self.acceleration_x.pop();
        self.acceleration_y.pop();
        self.acceleration_z.pop();
        
        self.color_r.pop();
        self.color_g.pop();
        self.color_b.pop();
        self.color_a.pop();
        
        self.size.pop();
        
        self.lifetime.pop();
        self.max_lifetime.pop();
        
        self.particle_type.pop();
        
        self.gravity_multiplier.pop();
        self.drag.pop();
        self.bounce.pop();
        
        self.rotation.pop();
        self.rotation_speed.pop();
        self.texture_frame.pop();
        self.animation_speed.pop();
        self.emissive.pop();
        self.emission_intensity.pop();
        
        self.size_curve_type.pop();
        self.size_curve_param1.pop();
        self.size_curve_param2.pop();
        self.size_curve_param3.pop();
        
        self.color_curve_type.pop();
        self.color_curve_param1.pop();
        self.color_curve_param2.pop();
        
        self.count -= 1;
    }
}

/// Particle pool for efficient allocation
pub struct ParticlePool {
    /// Pre-allocated particle data
    pub data: ParticleData,
    /// Next available index
    pub next_free: usize,
}

impl ParticlePool {
    /// Create a new particle pool
    pub fn new(capacity: usize) -> Self {
        Self {
            data: ParticleData::new(capacity),
            next_free: 0,
        }
    }
    
    /// Allocate space for new particles, returns start index and count allocated
    pub fn allocate(&mut self, count: usize) -> Option<(usize, usize)> {
        let available = self.data.count.saturating_sub(self.next_free);
        if available == 0 {
            return None;
        }
        
        let allocated = count.min(available);
        let start = self.next_free;
        self.next_free += allocated;
        
        Some((start, allocated))
    }
    
    /// Reset allocation pointer
    pub fn reset(&mut self) {
        self.next_free = 0;
    }
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

impl EmitterData {
    /// Create new emitter data buffer
    pub fn new(capacity: usize) -> Self {
        Self {
            count: 0,
            
            id: Vec::with_capacity(capacity),
            
            position_x: Vec::with_capacity(capacity),
            position_y: Vec::with_capacity(capacity),
            position_z: Vec::with_capacity(capacity),
            
            emission_rate: Vec::with_capacity(capacity),
            accumulated_particles: Vec::with_capacity(capacity),
            particle_type: Vec::with_capacity(capacity),
            
            elapsed_time: Vec::with_capacity(capacity),
            duration: Vec::with_capacity(capacity),
            
            shape_type: Vec::with_capacity(capacity),
            shape_param1: Vec::with_capacity(capacity),
            shape_param2: Vec::with_capacity(capacity),
            shape_param3: Vec::with_capacity(capacity),
            
            base_velocity_x: Vec::with_capacity(capacity),
            base_velocity_y: Vec::with_capacity(capacity),
            base_velocity_z: Vec::with_capacity(capacity),
            velocity_variance: Vec::with_capacity(capacity),
        }
    }
    
    /// Clear all emitter data
    pub fn clear(&mut self) {
        self.count = 0;
        
        self.id.clear();
        
        self.position_x.clear();
        self.position_y.clear();
        self.position_z.clear();
        
        self.emission_rate.clear();
        self.accumulated_particles.clear();
        self.particle_type.clear();
        
        self.elapsed_time.clear();
        self.duration.clear();
        
        self.shape_type.clear();
        self.shape_param1.clear();
        self.shape_param2.clear();
        self.shape_param3.clear();
        
        self.base_velocity_x.clear();
        self.base_velocity_y.clear();
        self.base_velocity_z.clear();
        self.velocity_variance.clear();
    }
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