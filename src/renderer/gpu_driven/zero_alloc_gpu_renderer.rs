/// Zero-allocation GPU-driven renderer optimizations for Sprint 37
/// Converts per-frame HashMap and Vec allocations to pre-allocated pools

use std::sync::Arc;
use std::collections::HashMap;
use cgmath::Vector3;
use crate::camera::data_camera::CameraData;
use crate::renderer::{GAME_POOLS, PooledVector, PooledHashMap};
use super::gpu_driven_renderer::RenderStats;

/// Pre-allocated render data structures to eliminate per-frame allocations
pub struct ZeroAllocRenderData {
    /// Pre-allocated instance buffers for each mesh type
    instance_buffers: Vec<Vec<u32>>,
    /// Pre-allocated map for grouping instances by mesh ID
    /// This replaces the HashMap::new() + Vec::new() allocations per frame
    mesh_instance_map_buffer: HashMap<u32, usize>, // mesh_id -> buffer_index
    
    /// Maximum number of mesh types we can handle without allocation
    max_mesh_types: usize,
    /// Current number of mesh types in use this frame
    active_mesh_types: usize,
}

impl ZeroAllocRenderData {
    pub fn new(max_mesh_types: usize, max_instances_per_mesh: usize) -> Self {
        let mut instance_buffers = Vec::with_capacity(max_mesh_types);
        for _ in 0..max_mesh_types {
            instance_buffers.push(Vec::with_capacity(max_instances_per_mesh));
        }
        
        Self {
            instance_buffers,
            mesh_instance_map_buffer: HashMap::with_capacity(max_mesh_types),
            max_mesh_types,
            active_mesh_types: 0,
        }
    }
    
    /// Clear all buffers for the next frame (no allocations)
    pub fn clear_for_frame(&mut self) {
        for buffer in &mut self.instance_buffers {
            buffer.clear();
        }
        self.mesh_instance_map_buffer.clear();
        self.active_mesh_types = 0;
    }
    
    /// Add an instance to a mesh group (zero allocation)
    pub fn add_instance(&mut self, mesh_id: u32, instance_idx: u32) -> Result<(), &'static str> {
        let buffer_index = if let Some(&existing_index) = self.mesh_instance_map_buffer.get(&mesh_id) {
            existing_index
        } else {
            if self.active_mesh_types >= self.max_mesh_types {
                return Err("Exceeded maximum mesh types per frame");
            }
            let new_index = self.active_mesh_types;
            self.mesh_instance_map_buffer.insert(mesh_id, new_index);
            self.active_mesh_types += 1;
            new_index
        };
        
        self.instance_buffers[buffer_index].push(instance_idx);
        Ok(())
    }
    
    /// Get instance buffer for a mesh ID
    pub fn get_instances(&self, mesh_id: u32) -> Option<&Vec<u32>> {
        let buffer_index = *self.mesh_instance_map_buffer.get(&mesh_id)?;
        Some(&self.instance_buffers[buffer_index])
    }
    
    /// Iterate over all active mesh groups
    pub fn iter_mesh_groups(&self) -> impl Iterator<Item = (u32, &Vec<u32>)> {
        self.mesh_instance_map_buffer
            .iter()
            .map(|(&mesh_id, &buffer_index)| (mesh_id, &self.instance_buffers[buffer_index]))
    }
}

/// Zero-allocation replacement for the GPU driven renderer's per-frame render loop
pub fn render_with_zero_allocations(
    render_data: &mut ZeroAllocRenderData,
    culling_metadata: &[super::culling_pipeline::DrawMetadata],
    gpu_meshing: &crate::renderer::gpu_meshing::GpuMeshingState,
    render_pass: &mut wgpu::RenderPass,
) -> Result<RenderStats, &'static str> {
    // Clear previous frame data (no allocations)
    render_data.clear_for_frame();
    
    let mut stats = RenderStats::default();
    
    // Group instances by mesh_id using pre-allocated buffers
    for (instance_idx, metadata) in culling_metadata.iter().enumerate() {
        stats.objects_submitted += 1;
        
        if metadata.flags & 1 != 0 { // Check visibility flag
            render_data.add_instance(metadata.mesh_id, instance_idx as u32)?;
        } else {
            stats.frustum_culled += 1;
        }
    }
    
    // Render each mesh group using pre-allocated data
    for (mesh_id, instance_indices) in render_data.iter_mesh_groups() {
        if let Some(mesh_buffer) = crate::renderer::gpu_meshing::get_mesh_buffer(gpu_meshing, mesh_id) {
            if !instance_indices.is_empty() {
                // Set GPU mesh buffers
                render_pass.set_vertex_buffer(0, mesh_buffer.vertices.slice(..));
                render_pass.set_index_buffer(mesh_buffer.indices.slice(..), wgpu::IndexFormat::Uint32);
                
                // Draw using fixed index count for now (36 indices for a cube)
                render_pass.draw_indexed(
                    0..36,
                    0,
                    0..instance_indices.len() as u32,
                );
                
                stats.draw_calls += 1;
                stats.objects_drawn += instance_indices.len() as u32;
            }
        }
    }
    
    Ok(stats)
}

/// Alternative approach using global pools for temporary allocations
pub fn render_with_pooled_collections(
    culling_metadata: &[super::culling_pipeline::DrawMetadata],
    gpu_meshing: &crate::renderer::gpu_meshing::GpuMeshingState,
    render_pass: &mut wgpu::RenderPass,
) -> Result<RenderStats, &'static str> {
    let mut stats = RenderStats::default();
    
    // Use pooled HashMap instead of HashMap::new()
    let mut instances_per_mesh = GAME_POOLS.chunk_pos_maps.acquire(16);
    
    // Collect instance IDs for each mesh (using pooled collections)
    for (instance_idx, metadata) in culling_metadata.iter().enumerate() {
        stats.objects_submitted += 1;
        
        if metadata.flags & 1 != 0 { // Check visibility flag
            // This still allocates Vec per mesh, but HashMap is pooled
            instances_per_mesh
                .entry(metadata.mesh_id)
                .or_insert_with(Vec::new)
                .push(instance_idx as u32);
        } else {
            stats.frustum_culled += 1;
        }
    }
    
    // Draw each mesh with its instances
    for (mesh_id, instance_indices) in instances_per_mesh.iter() {
        if let Some(mesh_buffer) = crate::renderer::gpu_meshing::get_mesh_buffer(gpu_meshing, *mesh_id) {
            if !instance_indices.is_empty() {
                // Set GPU mesh buffers
                render_pass.set_vertex_buffer(0, mesh_buffer.vertices.slice(..));
                render_pass.set_index_buffer(mesh_buffer.indices.slice(..), wgpu::IndexFormat::Uint32);
                
                // Draw using fixed index count for now (36 indices for a cube)
                render_pass.draw_indexed(
                    0..36,
                    0,
                    0..instance_indices.len() as u32,
                );
                
                stats.draw_calls += 1;
                stats.objects_drawn += instance_indices.len() as u32;
            }
        }
    }
    
    // HashMap is automatically returned to pool when dropped
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::culling_pipeline::DrawMetadata;
    
    #[test]
    fn test_zero_alloc_render_data() {
        let mut render_data = ZeroAllocRenderData::new(8, 100);
        
        // Add instances for different meshes
        assert!(render_data.add_instance(1, 10).is_ok());
        assert!(render_data.add_instance(1, 11).is_ok());
        assert!(render_data.add_instance(2, 20).is_ok());
        
        // Check that instances are grouped correctly
        let mesh1_instances = render_data.get_instances(1).expect("mesh1 instances should exist");
        assert_eq!(mesh1_instances, &vec![10, 11]);
        
        let mesh2_instances = render_data.get_instances(2).expect("mesh2 instances should exist");
        assert_eq!(mesh2_instances, &vec![20]);
        
        // Clear and verify empty
        render_data.clear_for_frame();
        assert!(render_data.get_instances(1).is_none());
        assert!(render_data.get_instances(2).is_none());
    }
    
    #[test]
    fn test_mesh_type_limit() {
        let mut render_data = ZeroAllocRenderData::new(2, 100);
        
        // Fill up to limit
        assert!(render_data.add_instance(1, 10).is_ok());
        assert!(render_data.add_instance(2, 20).is_ok());
        
        // Exceed limit
        assert!(render_data.add_instance(3, 30).is_err());
    }
}