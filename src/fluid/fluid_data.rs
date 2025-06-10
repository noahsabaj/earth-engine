use bytemuck::{Pod, Zeroable};
use wgpu::{Device, Buffer, BufferUsages};

/// Fluid types supported by the system
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidType {
    Air = 0,
    Water = 1,
    Lava = 2,
    Oil = 3,
    Steam = 4,
    Smoke = 5,
}

impl FluidType {
    /// Get fluid density (kg/m³)
    pub fn density(&self) -> f32 {
        match self {
            FluidType::Air => 1.2,
            FluidType::Water => 1000.0,
            FluidType::Lava => 3100.0,
            FluidType::Oil => 800.0,
            FluidType::Steam => 0.6,
            FluidType::Smoke => 0.3,
        }
    }
    
    /// Get fluid viscosity
    pub fn viscosity(&self) -> f32 {
        match self {
            FluidType::Air => 0.00001,
            FluidType::Water => 0.001,
            FluidType::Lava => 100.0,
            FluidType::Oil => 0.1,
            FluidType::Steam => 0.00001,
            FluidType::Smoke => 0.00001,
        }
    }
    
    /// Get temperature (Kelvin)
    pub fn temperature(&self) -> f32 {
        match self {
            FluidType::Air => 293.0,
            FluidType::Water => 293.0,
            FluidType::Lava => 1300.0,
            FluidType::Oil => 293.0,
            FluidType::Steam => 373.0,
            FluidType::Smoke => 400.0,
        }
    }
}

/// Fluid voxel data - packed for GPU efficiency
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct FluidVoxel {
    /// Packed fluid data:
    /// - Bits 0-7: Fluid type
    /// - Bits 8-15: Fluid level (0-255)
    /// - Bits 16-23: Temperature offset
    /// - Bits 24-31: Flags (solid neighbor bits, etc)
    pub packed_data: u32,
    
    /// Velocity vector (half precision would be better but using f32 for simplicity)
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub velocity_z: f32,
    
    /// Pressure value
    pub pressure: f32,
}

impl FluidVoxel {
    /// Create empty fluid voxel (air)
    pub fn empty() -> Self {
        Self {
            packed_data: 0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            velocity_z: 0.0,
            pressure: 0.0,
        }
    }
    
    /// Create fluid voxel with type and level
    pub fn new(fluid_type: FluidType, level: u8) -> Self {
        let packed_data = (fluid_type as u32) | ((level as u32) << 8);
        Self {
            packed_data,
            velocity_x: 0.0,
            velocity_y: 0.0,
            velocity_z: 0.0,
            pressure: 0.0,
        }
    }
    
    /// Get fluid type
    pub fn fluid_type(&self) -> FluidType {
        unsafe { std::mem::transmute((self.packed_data & 0xFF) as u8) }
    }
    
    /// Get fluid level (0-255)
    pub fn level(&self) -> u8 {
        ((self.packed_data >> 8) & 0xFF) as u8
    }
    
    /// Set fluid level
    pub fn set_level(&mut self, level: u8) {
        self.packed_data = (self.packed_data & !0xFF00) | ((level as u32) << 8);
    }
    
    /// Get temperature offset (-128 to 127)
    pub fn temperature_offset(&self) -> i8 {
        ((self.packed_data >> 16) & 0xFF) as i8
    }
    
    /// Check if cell has solid neighbor in direction
    pub fn has_solid_neighbor(&self, direction: Direction) -> bool {
        let bit = 24 + direction as u32;
        (self.packed_data & (1 << bit)) != 0
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    PosX = 0,
    NegX = 1,
    PosY = 2,
    NegY = 3,
    PosZ = 4,
    NegZ = 5,
}

/// GPU buffer for fluid simulation
pub struct FluidBuffer {
    /// Main fluid voxel buffer
    pub voxel_buffer: Buffer,
    
    /// Temporary buffer for double buffering
    pub temp_buffer: Buffer,
    
    /// Buffer size in voxels
    pub size: (u32, u32, u32),
    
    /// Total voxel count
    pub voxel_count: u32,
}

impl FluidBuffer {
    /// Create new fluid buffer
    pub fn new(device: &Device, size: (u32, u32, u32)) -> Self {
        let voxel_count = size.0 * size.1 * size.2;
        let buffer_size = (voxel_count as usize * std::mem::size_of::<FluidVoxel>()) as u64;
        
        let voxel_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fluid Voxel Buffer"),
            size: buffer_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        let temp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fluid Temp Buffer"),
            size: buffer_size,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        
        Self {
            voxel_buffer,
            temp_buffer,
            size,
            voxel_count,
        }
    }
    
    /// Get voxel index from coordinates
    pub fn voxel_index(&self, x: u32, y: u32, z: u32) -> u32 {
        x + y * self.size.0 + z * self.size.0 * self.size.1
    }
    
    /// Swap buffers for double buffering
    pub fn swap_buffers(&mut self) {
        std::mem::swap(&mut self.voxel_buffer, &mut self.temp_buffer);
    }
}

/// Fluid simulation constants for GPU
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct FluidConstants {
    /// World size in voxels
    pub world_size_x: u32,
    pub world_size_y: u32,
    pub world_size_z: u32,
    
    /// Time step
    pub dt: f32,
    
    /// Gravity
    pub gravity: f32,
    
    /// Number of pressure iterations
    pub pressure_iterations: u32,
    
    /// Fluid cell size
    pub cell_size: f32,
    
    /// Maximum velocity
    pub max_velocity: f32,
    
    /// Viscosity damping
    pub viscosity_damping: f32,
    
    /// Surface tension coefficient
    pub surface_tension: f32,
    
    /// Padding for alignment
    pub _padding: f32,
}

impl Default for FluidConstants {
    fn default() -> Self {
        Self {
            world_size_x: 256,
            world_size_y: 128,
            world_size_z: 256,
            dt: crate::fluid::FLUID_TIME_STEP,
            gravity: -9.81,
            pressure_iterations: crate::fluid::PRESSURE_ITERATIONS,
            cell_size: crate::fluid::FLUID_CELL_SIZE,
            max_velocity: crate::fluid::MAX_FLUID_VELOCITY,
            viscosity_damping: 0.98,
            surface_tension: 0.01,
            _padding: 0.0,
        }
    }
}

/// Fluid boundary conditions
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct BoundaryConditions {
    /// Boundary type for each face (0=open, 1=solid, 2=periodic)
    pub boundary_x_neg: u32,
    pub boundary_x_pos: u32,
    pub boundary_y_neg: u32,
    pub boundary_y_pos: u32,
    pub boundary_z_neg: u32,
    pub boundary_z_pos: u32,
    
    /// Padding
    pub _padding: [u32; 2],
}

impl Default for BoundaryConditions {
    fn default() -> Self {
        Self {
            boundary_x_neg: 1, // Solid walls by default
            boundary_x_pos: 1,
            boundary_y_neg: 1,
            boundary_y_pos: 0, // Open top
            boundary_z_neg: 1,
            boundary_z_pos: 1,
            _padding: [0; 2],
        }
    }
}

/// Fluid source/sink for spawning/removing fluid
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct FluidSource {
    /// Position in world space
    pub position: [f32; 3],
    
    /// Source radius
    pub radius: f32,
    
    /// Flow rate (voxels per second, negative for sink)
    pub flow_rate: f32,
    
    /// Fluid type to spawn
    pub fluid_type: u32,
    
    /// Initial velocity
    pub velocity: [f32; 3],
    
    /// Temperature
    pub temperature: f32,
    
    /// Active flag
    pub active: u32,
    
    /// Padding
    pub _padding: [u32; 3],
}