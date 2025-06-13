use wgpu::{Device, Buffer};
use crate::world_gpu::WorldBuffer;
use crate::sdf::{SdfChunk, SdfGenerator, SdfLod, LodLevel};
use std::sync::Arc;
use std::collections::HashMap;
use glam::{Vec3, IVec3};

/// Render mode selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderMode {
    /// Always render as voxels
    Voxel,
    
    /// Always render as smooth mesh
    Smooth,
    
    /// Automatically choose based on context
    Auto,
    
    /// Debug mode showing both
    Debug,
}

/// Dual representation of world data
pub struct DualRepresentation {
    /// Voxel data (source of truth)
    world_buffer: Arc<WorldBuffer>,
    
    /// SDF chunks
    sdf_chunks: HashMap<IVec3, SdfChunk>,
    
    /// Chunk size in voxels
    chunk_size: u32,
    
    /// Current render mode
    render_mode: RenderMode,
    
    /// SDF generator
    sdf_generator: SdfGenerator,
    
    /// LOD system
    lod_system: SdfLod,
    
    /// Update queue
    dirty_chunks: Vec<IVec3>,
    
    /// Device reference
    device: Arc<Device>,
}

impl DualRepresentation {
    /// Create new dual representation
    pub fn new(
        device: Arc<Device>,
        world_buffer: Arc<WorldBuffer>,
        chunk_size: u32,
    ) -> Self {
        let sdf_generator = SdfGenerator::new(device.clone());
        let lod_system = SdfLod::new(device.clone());
        
        Self {
            world_buffer,
            sdf_chunks: HashMap::new(),
            chunk_size,
            render_mode: RenderMode::Auto,
            sdf_generator,
            lod_system,
            dirty_chunks: Vec::new(),
            device,
        }
    }
    
    /// Set render mode
    pub fn set_render_mode(&mut self, mode: RenderMode) {
        self.render_mode = mode;
    }
    
    /// Mark chunk as needing SDF update
    pub fn mark_chunk_dirty(&mut self, chunk_pos: IVec3) {
        if !self.dirty_chunks.contains(&chunk_pos) {
            self.dirty_chunks.push(chunk_pos);
        }
        
        // Also mark neighboring chunks for smooth borders
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    let neighbor = chunk_pos + IVec3::new(dx, dy, dz);
                    if !self.dirty_chunks.contains(&neighbor) {
                        self.dirty_chunks.push(neighbor);
                    }
                }
            }
        }
    }
    
    /// Update dirty chunks
    pub fn update_dirty_chunks(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        max_updates: usize,
    ) {
        let updates = self.dirty_chunks.len().min(max_updates);
        
        for _ in 0..updates {
            if let Some(chunk_pos) = self.dirty_chunks.pop() {
                self.update_chunk_sdf(encoder, chunk_pos);
            }
        }
    }
    
    /// Update SDF for specific chunk
    fn update_chunk_sdf(&mut self, encoder: &mut wgpu::CommandEncoder, chunk_pos: IVec3) {
        // Get or create SDF chunk
        let sdf_chunk = self.sdf_chunks.entry(chunk_pos)
            .or_insert_with(|| {
                SdfChunk::new(
                    self.device.clone(),
                    (chunk_pos.x, chunk_pos.y, chunk_pos.z),
                    (self.chunk_size, self.chunk_size, self.chunk_size),
                )
            });
        
        // Generate SDF from voxel data
        let params = crate::sdf::SdfGenerationParams {
            chunk_offset: [chunk_pos.x, chunk_pos.y, chunk_pos.z],
            chunk_size: [self.chunk_size, self.chunk_size, self.chunk_size],
            sdf_size: sdf_chunk.sdf_buffer.size.into(),
            resolution: 1.0 / super::SDF_RESOLUTION_FACTOR,
            _padding: 0,
        };
        
        self.sdf_generator.generate(
            encoder,
            &self.world_buffer,
            &sdf_chunk.sdf_buffer,
            &params,
        );
        
        // Mark as no longer dirty but needs mesh generation
        if let Some(chunk) = self.sdf_chunks.get_mut(&chunk_pos) {
            chunk.dirty = false;
            chunk.has_surface = true; // TODO: Actual surface detection
        }
    }
    
    /// Get chunks to render based on camera position
    pub fn get_visible_chunks(
        &mut self,
        camera_pos: Vec3,
        view_distance: f32,
    ) -> Vec<ChunkRenderData> {
        let mut render_data = Vec::new();
        
        let camera_chunk = world_to_chunk_pos(camera_pos, self.chunk_size);
        let chunk_radius = (view_distance / self.chunk_size as f32).ceil() as i32;
        
        for x in -chunk_radius..=chunk_radius {
            for y in -chunk_radius..=chunk_radius {
                for z in -chunk_radius..=chunk_radius {
                    let chunk_pos = camera_chunk + IVec3::new(x, y, z);
                    
                    // Check if chunk exists in world
                    if !self.chunk_exists(chunk_pos) {
                        continue;
                    }
                    
                    // Determine render mode and LOD
                    let (mode, lod) = self.select_render_mode(chunk_pos, camera_pos);
                    
                    match mode {
                        RenderMode::Voxel => {
                            render_data.push(ChunkRenderData {
                                position: chunk_pos,
                                mode: RenderModeData::Voxel,
                                lod: LodLevel::Voxel,
                            });
                        }
                        RenderMode::Smooth => {
                            if let Some(chunk) = self.sdf_chunks.get(&chunk_pos) {
                                if chunk.has_surface {
                                    render_data.push(ChunkRenderData {
                                        position: chunk_pos,
                                        mode: RenderModeData::Smooth {
                                            vertices: chunk.mesh_vertices.clone(),
                                            indices: chunk.mesh_indices.clone(),
                                            vertex_count: chunk.vertex_count,
                                            index_count: chunk.index_count,
                                        },
                                        lod,
                                    });
                                }
                            }
                        }
                        RenderMode::Auto => {
                            // Handled by select_render_mode
                            unreachable!()
                        }
                        RenderMode::Debug => {
                            // Render both representations
                            render_data.push(ChunkRenderData {
                                position: chunk_pos,
                                mode: RenderModeData::Voxel,
                                lod: LodLevel::Voxel,
                            });
                            
                            if let Some(chunk) = self.sdf_chunks.get(&chunk_pos) {
                                if chunk.has_surface && chunk.mesh_vertices.is_some() {
                                    render_data.push(ChunkRenderData {
                                        position: chunk_pos,
                                        mode: RenderModeData::Smooth {
                                            vertices: chunk.mesh_vertices.clone(),
                                            indices: chunk.mesh_indices.clone(),
                                            vertex_count: chunk.vertex_count,
                                            index_count: chunk.index_count,
                                        },
                                        lod: LodLevel::High,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        render_data
    }
    
    /// Select render mode for chunk
    fn select_render_mode(&self, chunk_pos: IVec3, camera_pos: Vec3) -> (RenderMode, LodLevel) {
        if self.render_mode != RenderMode::Auto {
            return (self.render_mode, LodLevel::High);
        }
        
        // Calculate distance to chunk
        let chunk_world_pos = chunk_to_world_pos(chunk_pos, self.chunk_size);
        let distance = (chunk_world_pos - camera_pos).length();
        
        // Close chunks use voxel rendering for accuracy
        if distance < self.chunk_size as f32 * 2.0 {
            (RenderMode::Voxel, LodLevel::Voxel)
        } else {
            // Distant chunks use smooth rendering
            let lod = self.lod_system.select_lod(
                chunk_world_pos,
                self.chunk_size as f32,
                camera_pos,
                0.0,
            );
            (RenderMode::Smooth, lod)
        }
    }
    
    /// Check if chunk exists in world
    fn chunk_exists(&self, chunk_pos: IVec3) -> bool {
        // Simplified - would check world_buffer
        true
    }
    
    /// Generate all LODs for chunk
    pub fn generate_chunk_lods(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        chunk_pos: IVec3,
    ) {
        if let Some(chunk) = self.sdf_chunks.get_mut(&chunk_pos) {
            self.lod_system.generate_all_lods(encoder, chunk);
        }
    }
    
    /// Get memory usage statistics
    pub fn get_memory_usage(&self) -> MemoryStats {
        let mut stats = MemoryStats::default();
        
        // Voxel memory (estimated)
        stats.voxel_memory = self.world_buffer.buffer_size();
        
        // SDF memory
        for chunk in self.sdf_chunks.values() {
            stats.sdf_memory += chunk.sdf_buffer.size_bytes();
            
            if chunk.mesh_vertices.is_some() {
                stats.mesh_memory += (chunk.vertex_count as u64) * std::mem::size_of::<crate::sdf::SmoothVertex>() as u64;
                stats.mesh_memory += (chunk.index_count as u64) * std::mem::size_of::<u32>() as u64;
            }
        }
        
        stats.total_memory = stats.voxel_memory + stats.sdf_memory + stats.mesh_memory;
        
        stats
    }
}

/// Chunk render data
pub struct ChunkRenderData {
    /// Chunk position
    pub position: IVec3,
    
    /// Render mode data
    pub mode: RenderModeData,
    
    /// LOD level
    pub lod: LodLevel,
}

/// Render mode specific data
pub enum RenderModeData {
    /// Voxel rendering
    Voxel,
    
    /// Smooth mesh rendering
    Smooth {
        vertices: Option<Arc<Buffer>>,
        indices: Option<Arc<Buffer>>,
        vertex_count: u32,
        index_count: u32,
    },
}

/// Memory usage statistics
#[derive(Default, Debug)]
pub struct MemoryStats {
    /// Voxel buffer memory
    pub voxel_memory: u64,
    
    /// SDF buffer memory
    pub sdf_memory: u64,
    
    /// Mesh buffer memory
    pub mesh_memory: u64,
    
    /// Total memory usage
    pub total_memory: u64,
}

/// Convert world position to chunk position
fn world_to_chunk_pos(world_pos: Vec3, chunk_size: u32) -> IVec3 {
    IVec3::new(
        (world_pos.x / chunk_size as f32).floor() as i32,
        (world_pos.y / chunk_size as f32).floor() as i32,
        (world_pos.z / chunk_size as f32).floor() as i32,
    )
}

/// Convert chunk position to world position
fn chunk_to_world_pos(chunk_pos: IVec3, chunk_size: u32) -> Vec3 {
    Vec3::new(
        chunk_pos.x as f32 * chunk_size as f32,
        chunk_pos.y as f32 * chunk_size as f32,
        chunk_pos.z as f32 * chunk_size as f32,
    )
}

/// Transition settings
pub struct TransitionSettings {
    /// Distance to start transition
    pub start_distance: f32,
    
    /// Distance to complete transition
    pub end_distance: f32,
    
    /// Blend curve exponent
    pub blend_curve: f32,
}

impl Default for TransitionSettings {
    fn default() -> Self {
        Self {
            start_distance: 50.0,
            end_distance: 100.0,
            blend_curve: 2.0,
        }
    }
}