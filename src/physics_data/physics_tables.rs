use std::sync::atomic::{AtomicU32, Ordering};
use wgpu::util::DeviceExt;

/// Maximum number of physics entities
pub const MAX_ENTITIES: usize = 65536;

/// Entity identifier - simple index into arrays
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EntityId(pub u32);

impl EntityId {
    pub const INVALID: Self = Self(u32::MAX);
    
    pub fn is_valid(self) -> bool {
        self.0 < MAX_ENTITIES as u32
    }
    
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Axis-aligned bounding box for collision detection
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct AABB {
    pub min: [f32; 3],
    pub _pad1: f32,
    pub max: [f32; 3],
    pub _pad2: f32,
}

impl AABB {
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self {
            min,
            _pad1: 0.0,
            max,
            _pad2: 0.0,
        }
    }
    
    pub fn from_center_half_extents(center: [f32; 3], half_extents: [f32; 3]) -> Self {
        Self::new(
            [
                center[0] - half_extents[0],
                center[1] - half_extents[1],
                center[2] - half_extents[2],
            ],
            [
                center[0] + half_extents[0],
                center[1] + half_extents[1],
                center[2] + half_extents[2],
            ],
        )
    }
    
    pub fn intersects(&self, other: &AABB) -> bool {
        self.min[0] <= other.max[0] && self.max[0] >= other.min[0] &&
        self.min[1] <= other.max[1] && self.max[1] >= other.min[1] &&
        self.min[2] <= other.max[2] && self.max[2] >= other.min[2]
    }
}

/// Main physics data storage using struct-of-arrays
pub struct PhysicsData {
    // Entity count
    entity_count: AtomicU32,
    
    // Transform data
    pub positions: Vec<[f32; 3]>,
    pub velocities: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>, // Quaternions
    pub angular_velocities: Vec<[f32; 3]>,
    
    // Physical properties
    pub masses: Vec<f32>,
    pub inverse_masses: Vec<f32>, // Pre-computed for efficiency
    pub restitutions: Vec<f32>,
    pub frictions: Vec<f32>,
    
    // Collision data
    pub bounding_boxes: Vec<AABB>,
    pub collision_groups: Vec<u32>,
    pub collision_masks: Vec<u32>,
    
    // Status flags (packed for cache efficiency)
    pub flags: Vec<PhysicsFlags>,
    
    // GPU buffers (optional)
    pub gpu_buffers: Option<GpuPhysicsBuffers>,
}

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct PhysicsFlags {
    bits: u32,
}

impl PhysicsFlags {
    pub const ACTIVE: u32 = 1 << 0;
    pub const STATIC: u32 = 1 << 1;
    pub const KINEMATIC: u32 = 1 << 2;
    pub const GRAVITY: u32 = 1 << 3;
    pub const SLEEPING: u32 = 1 << 4;
    
    pub fn new() -> Self {
        Self { bits: Self::ACTIVE | Self::GRAVITY }
    }
    
    pub fn is_active(self) -> bool {
        (self.bits & Self::ACTIVE) != 0
    }
    
    pub fn is_static(self) -> bool {
        (self.bits & Self::STATIC) != 0
    }
    
    pub fn is_dynamic(self) -> bool {
        !self.is_static() && !self.is_kinematic()
    }
    
    pub fn is_kinematic(self) -> bool {
        (self.bits & Self::KINEMATIC) != 0
    }
    
    pub fn has_gravity(self) -> bool {
        (self.bits & Self::GRAVITY) != 0
    }
    
    pub fn is_sleeping(self) -> bool {
        (self.bits & Self::SLEEPING) != 0
    }
    
    pub fn set_flag(&mut self, flag: u32, value: bool) {
        if value {
            self.bits |= flag;
        } else {
            self.bits &= !flag;
        }
    }
}

/// GPU buffers for physics data
pub struct GpuPhysicsBuffers {
    pub position_buffer: wgpu::Buffer,
    pub velocity_buffer: wgpu::Buffer,
    pub mass_buffer: wgpu::Buffer,
    pub aabb_buffer: wgpu::Buffer,
    pub flags_buffer: wgpu::Buffer,
}

impl PhysicsData {
    pub fn new(max_entities: usize) -> Self {
        Self {
            entity_count: AtomicU32::new(0),
            
            // Initialize all arrays with capacity
            positions: Vec::with_capacity(max_entities),
            velocities: Vec::with_capacity(max_entities),
            rotations: Vec::with_capacity(max_entities),
            angular_velocities: Vec::with_capacity(max_entities),
            
            masses: Vec::with_capacity(max_entities),
            inverse_masses: Vec::with_capacity(max_entities),
            restitutions: Vec::with_capacity(max_entities),
            frictions: Vec::with_capacity(max_entities),
            
            bounding_boxes: Vec::with_capacity(max_entities),
            collision_groups: Vec::with_capacity(max_entities),
            collision_masks: Vec::with_capacity(max_entities),
            
            flags: Vec::with_capacity(max_entities),
            
            gpu_buffers: None,
        }
    }
    
    /// Add a new entity to the physics system
    pub fn add_entity(
        &mut self,
        position: [f32; 3],
        velocity: [f32; 3],
        mass: f32,
        half_extents: [f32; 3],
    ) -> EntityId {
        let id = self.entity_count.fetch_add(1, Ordering::SeqCst);
        assert!(id < MAX_ENTITIES as u32, "Too many physics entities");
        
        // Add to all arrays
        self.positions.push(position);
        self.velocities.push(velocity);
        self.rotations.push([0.0, 0.0, 0.0, 1.0]); // Identity quaternion
        self.angular_velocities.push([0.0, 0.0, 0.0]);
        
        self.masses.push(mass);
        self.inverse_masses.push(if mass > 0.0 { 1.0 / mass } else { 0.0 });
        self.restitutions.push(0.3); // Default restitution
        self.frictions.push(0.5); // Default friction
        
        self.bounding_boxes.push(AABB::from_center_half_extents(position, half_extents));
        self.collision_groups.push(1); // Default group
        self.collision_masks.push(u32::MAX); // Collide with everything
        
        self.flags.push(PhysicsFlags::new());
        
        EntityId(id)
    }
    
    /// Remove an entity (swap-remove for efficiency)
    pub fn remove_entity(&mut self, entity: EntityId) {
        if !entity.is_valid() {
            return;
        }
        
        let idx = entity.index();
        let last_idx = self.entity_count() - 1;
        
        if idx < last_idx {
            // Swap with last element
            self.positions.swap(idx, last_idx);
            self.velocities.swap(idx, last_idx);
            self.rotations.swap(idx, last_idx);
            self.angular_velocities.swap(idx, last_idx);
            
            self.masses.swap(idx, last_idx);
            self.inverse_masses.swap(idx, last_idx);
            self.restitutions.swap(idx, last_idx);
            self.frictions.swap(idx, last_idx);
            
            self.bounding_boxes.swap(idx, last_idx);
            self.collision_groups.swap(idx, last_idx);
            self.collision_masks.swap(idx, last_idx);
            
            self.flags.swap(idx, last_idx);
        }
        
        // Remove last element
        self.positions.pop();
        self.velocities.pop();
        self.rotations.pop();
        self.angular_velocities.pop();
        
        self.masses.pop();
        self.inverse_masses.pop();
        self.restitutions.pop();
        self.frictions.pop();
        
        self.bounding_boxes.pop();
        self.collision_groups.pop();
        self.collision_masks.pop();
        
        self.flags.pop();
        
        self.entity_count.fetch_sub(1, Ordering::SeqCst);
    }
    
    /// Get current entity count
    pub fn entity_count(&self) -> usize {
        self.entity_count.load(Ordering::SeqCst) as usize
    }
    
    /// Create GPU buffers for physics data
    pub fn create_gpu_buffers(&mut self, device: &wgpu::Device) {
        let count = self.entity_count();
        
        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Physics Position Buffer"),
            contents: bytemuck::cast_slice(&self.positions[..count]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        let velocity_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Physics Velocity Buffer"),
            contents: bytemuck::cast_slice(&self.velocities[..count]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        let mass_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Physics Mass Buffer"),
            contents: bytemuck::cast_slice(&self.masses[..count]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        let aabb_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Physics AABB Buffer"),
            contents: bytemuck::cast_slice(&self.bounding_boxes[..count]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        let flags_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Physics Flags Buffer"),
            contents: bytemuck::cast_slice(&self.flags[..count]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        
        self.gpu_buffers = Some(GpuPhysicsBuffers {
            position_buffer,
            velocity_buffer,
            mass_buffer,
            aabb_buffer,
            flags_buffer,
        });
    }
    
    /// Update GPU buffers with current data
    pub fn update_gpu_buffers(&self, queue: &wgpu::Queue) {
        if let Some(buffers) = &self.gpu_buffers {
            let count = self.entity_count();
            
            queue.write_buffer(
                &buffers.position_buffer,
                0,
                bytemuck::cast_slice(&self.positions[..count]),
            );
            
            queue.write_buffer(
                &buffers.velocity_buffer,
                0,
                bytemuck::cast_slice(&self.velocities[..count]),
            );
            
            queue.write_buffer(
                &buffers.mass_buffer,
                0,
                bytemuck::cast_slice(&self.masses[..count]),
            );
            
            queue.write_buffer(
                &buffers.aabb_buffer,
                0,
                bytemuck::cast_slice(&self.bounding_boxes[..count]),
            );
            
            queue.write_buffer(
                &buffers.flags_buffer,
                0,
                bytemuck::cast_slice(&self.flags[..count]),
            );
        }
    }
    
    /// Update bounding box for an entity based on its position
    pub fn update_bounding_box(&mut self, entity: EntityId, half_extents: [f32; 3]) {
        if let Some(idx) = entity.is_valid().then(|| entity.index()) {
            if idx < self.entity_count() {
                let pos = self.positions[idx];
                self.bounding_boxes[idx] = AABB::from_center_half_extents(pos, half_extents);
            }
        }
    }
    
    /// Clear all entities
    pub fn clear(&mut self) {
        self.positions.clear();
        self.velocities.clear();
        self.rotations.clear();
        self.angular_velocities.clear();
        
        self.masses.clear();
        self.inverse_masses.clear();
        self.restitutions.clear();
        self.frictions.clear();
        
        self.bounding_boxes.clear();
        self.collision_groups.clear();
        self.collision_masks.clear();
        
        self.flags.clear();
        
        self.entity_count.store(0, Ordering::SeqCst);
    }
}