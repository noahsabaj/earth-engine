use std::collections::HashMap;
use wgpu::util::DeviceExt;
use cgmath::{Vector4, Point3};
use crate::{
    world::{ChunkPos, Chunk, BlockRegistry, World},
    renderer::ChunkMesher,
    Camera,
};

pub struct ChunkRenderer {
    chunk_meshes: HashMap<ChunkPos, ChunkRenderData>,
}

struct ChunkRenderData {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

impl ChunkRenderer {
    pub fn new() -> Self {
        Self {
            chunk_meshes: HashMap::new(),
        }
    }

    pub fn update_chunk(
        &mut self,
        device: &wgpu::Device,
        chunk_pos: ChunkPos,
        chunk: &Chunk,
        registry: &BlockRegistry,
    ) {
        let mesh = ChunkMesher::generate_mesh(chunk, registry);
        
        if mesh.vertices.is_empty() {
            // Remove empty chunks
            self.chunk_meshes.remove(&chunk_pos);
            return;
        }
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Chunk {:?} Vertex Buffer", chunk_pos)),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("Chunk {:?} Index Buffer", chunk_pos)),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        self.chunk_meshes.insert(chunk_pos, ChunkRenderData {
            vertex_buffer,
            index_buffer,
            num_indices: mesh.indices.len() as u32,
        });
    }
    
    pub fn update_dirty_chunks(
        &mut self,
        device: &wgpu::Device,
        world: &mut World,
        registry: &BlockRegistry,
    ) {
        // Get dirty chunks from the world
        let dirty_chunks = world.take_dirty_chunks();
        
        // Update each dirty chunk
        for chunk_pos in dirty_chunks {
            if let Some(chunk) = world.get_chunk(chunk_pos) {
                self.update_chunk(device, chunk_pos, chunk, registry);
            }
            // Mark chunk as clean
            if let Some(chunk) = world.get_chunk_mut(chunk_pos) {
                chunk.mark_clean();
            }
        }
        
        // Remove meshes for unloaded chunks
        let loaded_positions: std::collections::HashSet<_> = world.chunks().keys().cloned().collect();
        self.chunk_meshes.retain(|pos, _| loaded_positions.contains(pos));
    }
    
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        for (_, chunk_data) in &self.chunk_meshes {
            render_pass.set_vertex_buffer(0, chunk_data.vertex_buffer.slice(..));
            render_pass.set_index_buffer(chunk_data.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..chunk_data.num_indices, 0, 0..1);
        }
    }
    
    pub fn render_with_frustum_culling<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera: &Camera,
        chunk_size: u32,
    ) {
        let view_proj = camera.build_projection_matrix() * camera.build_view_matrix();
        
        for (chunk_pos, chunk_data) in &self.chunk_meshes {
            // Calculate chunk bounds
            let min = Point3::new(
                (chunk_pos.x * chunk_size as i32) as f32,
                (chunk_pos.y * chunk_size as i32) as f32,
                (chunk_pos.z * chunk_size as i32) as f32,
            );
            let max = Point3::new(
                min.x + chunk_size as f32,
                min.y + chunk_size as f32,
                min.z + chunk_size as f32,
            );
            
            // Simple frustum culling - check if chunk center is in view
            let center = Point3::new(
                (min.x + max.x) * 0.5,
                (min.y + max.y) * 0.5,
                (min.z + max.z) * 0.5,
            );
            
            let clip_pos = view_proj * Vector4::new(center.x, center.y, center.z, 1.0);
            
            // Check if in frustum (rough check)
            if clip_pos.w > 0.0 {
                let ndc_x = clip_pos.x / clip_pos.w;
                let ndc_y = clip_pos.y / clip_pos.w;
                let ndc_z = clip_pos.z / clip_pos.w;
                
                // Expand bounds a bit to avoid culling edge chunks
                if ndc_x >= -1.5 && ndc_x <= 1.5 &&
                   ndc_y >= -1.5 && ndc_y <= 1.5 &&
                   ndc_z >= 0.0 && ndc_z <= 1.0 {
                    render_pass.set_vertex_buffer(0, chunk_data.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(chunk_data.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..chunk_data.num_indices, 0, 0..1);
                }
            }
        }
    }
}