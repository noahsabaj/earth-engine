/// SOA-based Mesh Builder for Cache-Efficient Mesh Generation
/// 
/// This module implements mesh building using Structure-of-Arrays patterns
/// for optimal cache performance during mesh generation and processing.

use super::vertex_soa::VertexBufferSoA;
use crate::BlockId;
use std::collections::HashMap;

/// Mesh generation data in SOA layout
pub struct MeshBuilderSoA {
    /// Vertex data arrays
    pub positions: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub light_levels: Vec<f32>,
    pub ao_values: Vec<f32>,
    
    /// Index data
    pub indices: Vec<u32>,
    
    /// Temporary working arrays (reused across chunks)
    temp_positions: Vec<[f32; 3]>,
    temp_normals: Vec<[f32; 3]>,
    temp_colors: Vec<[f32; 3]>,
    
    /// Face visibility cache (for greedy meshing)
    face_visibility: Vec<bool>,
    
    /// Block color lookup (pre-computed for cache efficiency)
    block_colors: HashMap<BlockId, [f32; 3]>,
}

impl MeshBuilderSoA {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            colors: Vec::new(),
            normals: Vec::new(),
            light_levels: Vec::new(),
            ao_values: Vec::new(),
            indices: Vec::new(),
            temp_positions: Vec::new(),
            temp_normals: Vec::new(),
            temp_colors: Vec::new(),
            face_visibility: Vec::new(),
            block_colors: Self::init_block_colors(),
        }
    }
    
    /// Pre-compute block colors for cache efficiency
    fn init_block_colors() -> HashMap<BlockId, [f32; 3]> {
        let mut colors = HashMap::new();
        colors.insert(BlockId::AIR, [0.0, 0.0, 0.0]);
        colors.insert(BlockId::GRASS, [0.4, 0.8, 0.2]);
        colors.insert(BlockId::DIRT, [0.6, 0.4, 0.2]);
        colors.insert(BlockId::STONE, [0.5, 0.5, 0.5]);
        colors.insert(BlockId::WOOD, [0.6, 0.3, 0.1]);
        colors.insert(BlockId::SAND, [0.9, 0.8, 0.6]);
        colors.insert(BlockId::WATER, [0.2, 0.4, 0.8]);
        colors.insert(BlockId::LAVA, [1.0, 0.3, 0.0]);
        colors
    }
    
    /// Clear all mesh data
    pub fn clear(&mut self) {
        self.positions.clear();
        self.colors.clear();
        self.normals.clear();
        self.light_levels.clear();
        self.ao_values.clear();
        self.indices.clear();
        
        // Keep temp arrays allocated but clear them
        self.temp_positions.clear();
        self.temp_normals.clear();
        self.temp_colors.clear();
        self.face_visibility.clear();
    }
    
    /// Reserve capacity for expected vertex count
    pub fn reserve(&mut self, vertex_count: usize) {
        self.positions.reserve(vertex_count);
        self.colors.reserve(vertex_count);
        self.normals.reserve(vertex_count);
        self.light_levels.reserve(vertex_count);
        self.ao_values.reserve(vertex_count);
        self.indices.reserve(vertex_count / 4 * 6); // Rough estimate for quads
    }
    
    /// Add a quad to the mesh (cache-friendly batch operation)
    pub fn add_quad_soa(
        &mut self,
        quad_positions: [[f32; 3]; 4],
        normal: [f32; 3],
        block_id: BlockId,
        light: f32,
        ao_values: [f32; 4],
    ) {
        let base_index = self.positions.len() as u32;
        let color = self.block_colors.get(&block_id).copied().unwrap_or([1.0, 0.0, 1.0]);
        
        // Add vertices in batch (cache-friendly)
        for i in 0..4 {
            self.positions.push(quad_positions[i]);
            self.colors.push(color);
            self.normals.push(normal);
            self.light_levels.push(light);
            self.ao_values.push(ao_values[i]);
        }
        
        // Add indices for two triangles
        self.indices.extend_from_slice(&[
            base_index, base_index + 1, base_index + 2,
            base_index, base_index + 2, base_index + 3,
        ]);
    }
    
    /// Batch add multiple quads (more cache-efficient)
    pub fn add_quads_batch<I>(&mut self, quads: I)
    where
        I: Iterator<Item = ([[f32; 3]; 4], [f32; 3], BlockId, f32, [f32; 4])>,
    {
        // Collect into temporary arrays first for better memory access patterns
        self.temp_positions.clear();
        self.temp_normals.clear();
        self.temp_colors.clear();
        
        let mut temp_light_levels = Vec::new();
        let mut temp_ao_values = Vec::new();
        let mut temp_indices = Vec::new();
        
        for (i, (quad_positions, normal, block_id, light, ao_values)) in quads.enumerate() {
            let base_index = (self.positions.len() + i * 4) as u32;
            let color = self.block_colors.get(&block_id).copied().unwrap_or([1.0, 0.0, 1.0]);
            
            // Collect vertices
            for j in 0..4 {
                self.temp_positions.push(quad_positions[j]);
                self.temp_normals.push(normal);
                self.temp_colors.push(color);
                temp_light_levels.push(light);
                temp_ao_values.push(ao_values[j]);
            }
            
            // Collect indices
            temp_indices.extend_from_slice(&[
                base_index, base_index + 1, base_index + 2,
                base_index, base_index + 2, base_index + 3,
            ]);
        }
        
        // Batch append to main arrays (better cache behavior)
        self.positions.extend_from_slice(&self.temp_positions);
        self.colors.extend_from_slice(&self.temp_colors);
        self.normals.extend_from_slice(&self.temp_normals);
        self.light_levels.extend_from_slice(&temp_light_levels);
        self.ao_values.extend_from_slice(&temp_ao_values);
        self.indices.extend_from_slice(&temp_indices);
    }
    
    /// Convert to VertexBufferSoA for GPU upload
    pub fn build_vertex_buffer(&self) -> VertexBufferSoA {
        let mut vertex_buffer = VertexBufferSoA::new();
        
        // Batch copy all vertex data
        for i in 0..self.positions.len() {
            vertex_buffer.push(
                self.positions[i],
                self.colors[i],
                self.normals[i],
                self.light_levels[i],
                self.ao_values[i],
            );
        }
        
        vertex_buffer
    }
    
    /// Get current vertex count
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }
    
    /// Get current index count
    pub fn index_count(&self) -> usize {
        self.indices.len()
    }
    
    /// Get memory statistics
    pub fn memory_stats(&self) -> MeshBuilderStats {
        MeshBuilderStats {
            vertex_count: self.vertex_count(),
            index_count: self.index_count(),
            positions_bytes: self.positions.len() * std::mem::size_of::<[f32; 3]>(),
            colors_bytes: self.colors.len() * std::mem::size_of::<[f32; 3]>(),
            normals_bytes: self.normals.len() * std::mem::size_of::<[f32; 3]>(),
            light_bytes: self.light_levels.len() * std::mem::size_of::<f32>(),
            ao_bytes: self.ao_values.len() * std::mem::size_of::<f32>(),
            indices_bytes: self.indices.len() * std::mem::size_of::<u32>(),
        }
    }
}

/// Memory usage statistics for mesh builder
#[derive(Debug, Clone)]
pub struct MeshBuilderStats {
    pub vertex_count: usize,
    pub index_count: usize,
    pub positions_bytes: usize,
    pub colors_bytes: usize,
    pub normals_bytes: usize,
    pub light_bytes: usize,
    pub ao_bytes: usize,
    pub indices_bytes: usize,
}

impl MeshBuilderStats {
    pub fn total_bytes(&self) -> usize {
        self.positions_bytes + 
        self.colors_bytes + 
        self.normals_bytes + 
        self.light_bytes + 
        self.ao_bytes + 
        self.indices_bytes
    }
}

/// Greedy meshing using SOA for cache efficiency
pub struct GreedyMeshBuilderSoA {
    builder: MeshBuilderSoA,
    /// Chunk size for greedy meshing
    chunk_size: usize,
    /// Visited mask for greedy algorithm
    visited: Vec<bool>,
}

impl GreedyMeshBuilderSoA {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            builder: MeshBuilderSoA::new(),
            chunk_size,
            visited: vec![false; chunk_size * chunk_size * chunk_size],
        }
    }
    
    /// Build mesh using greedy algorithm with SOA data
    pub fn build_greedy_mesh(
        &mut self,
        blocks: &[BlockId],
        light_data: &[u8],
        chunk_size: usize,
    ) -> VertexBufferSoA {
        self.builder.clear();
        self.visited.fill(false);
        
        // Process each face direction for greedy meshing
        for axis in 0..3 {
            for direction in 0..2 {
                self.build_greedy_quads_for_axis(blocks, light_data, chunk_size, axis, direction);
            }
        }
        
        self.builder.build_vertex_buffer()
    }
    
    /// Build greedy quads for a specific axis and direction
    fn build_greedy_quads_for_axis(
        &mut self,
        blocks: &[BlockId],
        light_data: &[u8],
        chunk_size: usize,
        axis: usize,
        direction: usize,
    ) {
        let (u_axis, v_axis) = match axis {
            0 => (1, 2), // X axis: U=Y, V=Z
            1 => (0, 2), // Y axis: U=X, V=Z
            _ => (0, 1), // Z axis: U=X, V=Y
        };
        
        for layer in 0..chunk_size {
            self.build_layer_quads(blocks, light_data, chunk_size, axis, direction, layer, u_axis, v_axis);
        }
    }
    
    /// Build quads for a single layer (optimized with SOA access patterns)
    fn build_layer_quads(
        &mut self,
        blocks: &[BlockId],
        light_data: &[u8],
        chunk_size: usize,
        axis: usize,
        direction: usize,
        layer: usize,
        u_axis: usize,
        v_axis: usize,
    ) {
        // Reset visited for this layer
        for u in 0..chunk_size {
            for v in 0..chunk_size {
                let index = self.get_block_index(axis, layer, u, v, chunk_size, u_axis, v_axis);
                if index < self.visited.len() {
                    self.visited[index] = false;
                }
            }
        }
        
        // Find and build quads using greedy algorithm
        for u in 0..chunk_size {
            for v in 0..chunk_size {
                let index = self.get_block_index(axis, layer, u, v, chunk_size, u_axis, v_axis);
                
                if index >= blocks.len() || self.visited[index] {
                    continue;
                }
                
                let block = blocks[index];
                if block == BlockId::AIR {
                    continue;
                }
                
                // Check if face should be rendered
                if !self.should_render_face(blocks, chunk_size, axis, direction, layer, u, v, u_axis, v_axis) {
                    continue;
                }
                
                // Find the largest possible quad starting from this position
                let (width, height) = self.find_quad_size(
                    blocks, chunk_size, axis, layer, u, v, u_axis, v_axis, block
                );
                
                // Mark visited area
                for du in 0..width {
                    for dv in 0..height {
                        let visit_index = self.get_block_index(
                            axis, layer, u + du, v + dv, chunk_size, u_axis, v_axis
                        );
                        if visit_index < self.visited.len() {
                            self.visited[visit_index] = true;
                        }
                    }
                }
                
                // Generate quad
                self.generate_quad(
                    axis, direction, layer, u, v, width, height,
                    block, light_data, chunk_size, u_axis, v_axis
                );
            }
        }
    }
    
    /// Get block index for 3D coordinates
    fn get_block_index(
        &self,
        axis: usize,
        layer: usize,
        u: usize,
        v: usize,
        chunk_size: usize,
        u_axis: usize,
        v_axis: usize,
    ) -> usize {
        let mut coords = [0; 3];
        coords[axis] = layer;
        coords[u_axis] = u;
        coords[v_axis] = v;
        
        coords[0] + coords[1] * chunk_size + coords[2] * chunk_size * chunk_size
    }
    
    /// Check if a face should be rendered
    fn should_render_face(
        &self,
        blocks: &[BlockId],
        chunk_size: usize,
        axis: usize,
        direction: usize,
        layer: usize,
        u: usize,
        v: usize,
        u_axis: usize,
        v_axis: usize,
    ) -> bool {
        // Check adjacent block
        let neighbor_layer = if direction == 0 {
            if layer == 0 { return true; }
            layer - 1
        } else {
            if layer == chunk_size - 1 { return true; }
            layer + 1
        };
        
        let neighbor_index = self.get_block_index(axis, neighbor_layer, u, v, chunk_size, u_axis, v_axis);
        if neighbor_index >= blocks.len() {
            return true;
        }
        
        blocks[neighbor_index] == BlockId::AIR
    }
    
    /// Find the largest possible quad size
    fn find_quad_size(
        &self,
        blocks: &[BlockId],
        chunk_size: usize,
        axis: usize,
        layer: usize,
        start_u: usize,
        start_v: usize,
        u_axis: usize,
        v_axis: usize,
        block_type: BlockId,
    ) -> (usize, usize) {
        // Find width (expand in U direction)
        let mut width = 1;
        while start_u + width < chunk_size {
            let index = self.get_block_index(axis, layer, start_u + width, start_v, chunk_size, u_axis, v_axis);
            if index >= blocks.len() || 
               self.visited[index] || 
               blocks[index] != block_type {
                break;
            }
            width += 1;
        }
        
        // Find height (expand in V direction)
        let mut height = 1;
        'height_loop: while start_v + height < chunk_size {
            // Check entire row at this height
            for u_offset in 0..width {
                let index = self.get_block_index(
                    axis, layer, start_u + u_offset, start_v + height, chunk_size, u_axis, v_axis
                );
                if index >= blocks.len() || 
                   self.visited[index] || 
                   blocks[index] != block_type {
                    break 'height_loop;
                }
            }
            height += 1;
        }
        
        (width, height)
    }
    
    /// Generate a quad with the given parameters
    fn generate_quad(
        &mut self,
        axis: usize,
        direction: usize,
        layer: usize,
        u: usize,
        v: usize,
        width: usize,
        height: usize,
        block: BlockId,
        light_data: &[u8],
        chunk_size: usize,
        u_axis: usize,
        v_axis: usize,
    ) {
        // Calculate quad positions
        let mut positions = [[0.0f32; 3]; 4];
        
        // Base position
        positions[0][axis] = layer as f32;
        positions[0][u_axis] = u as f32;
        positions[0][v_axis] = v as f32;
        
        // Adjust for direction
        if direction == 1 {
            positions[0][axis] += 1.0;
        }
        
        // Create quad vertices
        positions[1] = positions[0];
        positions[1][u_axis] += width as f32;
        
        positions[2] = positions[1];
        positions[2][v_axis] += height as f32;
        
        positions[3] = positions[0];
        positions[3][v_axis] += height as f32;
        
        // Calculate normal
        let mut normal = [0.0f32; 3];
        normal[axis] = if direction == 0 { -1.0 } else { 1.0 };
        
        // Get light level (sample from center of quad)
        let light_index = self.get_block_index(
            axis, layer, u + width / 2, v + height / 2, chunk_size, u_axis, v_axis
        );
        let light = if light_index < light_data.len() {
            light_data[light_index] as f32 / 15.0
        } else {
            1.0
        };
        
        // Generate AO values (simplified for greedy meshing)
        let ao_values = [1.0, 1.0, 1.0, 1.0];
        
        // Add quad to builder
        self.builder.add_quad_soa(positions, normal, block, light, ao_values);
    }
    
    /// Get mesh builder statistics
    pub fn stats(&self) -> MeshBuilderStats {
        self.builder.memory_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_soa_mesh_builder() {
        let mut builder = MeshBuilderSoA::new();
        
        let positions = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let normal = [0.0, 0.0, 1.0];
        let ao_values = [1.0, 0.8, 0.6, 0.9];
        
        builder.add_quad_soa(positions, normal, BlockId::STONE, 1.0, ao_values);
        
        assert_eq!(builder.vertex_count(), 4);
        assert_eq!(builder.index_count(), 6);
    }
    
    #[test]
    fn test_greedy_mesh_builder() {
        let mut builder = GreedyMeshBuilderSoA::new(4);
        
        // Create a simple 4x4x4 chunk with some blocks
        let mut blocks = vec![BlockId::AIR; 64];
        for i in 0..16 {
            blocks[i] = BlockId::STONE; // Bottom layer
        }
        
        let light_data = vec![15u8; 64]; // Full light
        
        let vertex_buffer = builder.build_greedy_mesh(&blocks, &light_data, 4);
        
        // Should have generated some vertices for the stone blocks
        assert!(vertex_buffer.len() > 0);
    }
}