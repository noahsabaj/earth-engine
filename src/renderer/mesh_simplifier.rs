/// Mesh Simplification System
/// 
/// Reduces mesh complexity for distant chunks using quadric error metrics.
/// Preserves visual quality while dramatically reducing triangle count.
/// Part of Sprint 29: Mesh Optimization & Advanced LOD

use cgmath::{Vector3, Matrix4, Zero};
use crate::renderer::Vertex;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;

/// Quadric error matrix for vertex
#[derive(Debug, Clone, Copy)]
struct Quadric {
    matrix: Matrix4<f32>,
}

impl Quadric {
    fn new() -> Self {
        Self {
            matrix: Matrix4::zero(),
        }
    }
    
    /// Create quadric from plane equation
    fn from_plane(a: f32, b: f32, c: f32, d: f32) -> Self {
        let matrix = Matrix4::new(
            a * a, a * b, a * c, a * d,
            a * b, b * b, b * c, b * d,
            a * c, b * c, c * c, c * d,
            a * d, b * d, c * d, d * d,
        );
        Self { matrix }
    }
    
    /// Add two quadrics
    fn add(&self, other: &Quadric) -> Quadric {
        Quadric {
            matrix: self.matrix + other.matrix,
        }
    }
    
    /// Compute error for vertex position
    fn compute_error(&self, pos: Vector3<f32>) -> f32 {
        let v = cgmath::Vector4::new(pos.x, pos.y, pos.z, 1.0);
        let result = self.matrix * v;
        v.dot(result).abs()
    }
}

/// Edge collapse candidate
#[derive(Debug, Clone)]
struct CollapseCandidate {
    edge: (u32, u32),
    error: f32,
    target_position: Vector3<f32>,
}

impl PartialEq for CollapseCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.error.eq(&other.error)
    }
}

impl Eq for CollapseCandidate {}

impl PartialOrd for CollapseCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse order for min-heap behavior
        other.error.partial_cmp(&self.error)
    }
}

impl Ord for CollapseCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// Mesh simplifier using quadric error metrics
pub struct MeshSimplifier {
    /// Vertex quadrics
    vertex_quadrics: Vec<Quadric>,
    
    /// Face list for topology
    faces: Vec<[u32; 3]>,
    
    /// Vertex positions
    positions: Vec<Vector3<f32>>,
    
    /// Valid edges
    edges: HashSet<(u32, u32)>,
    
    /// Vertex to faces mapping
    vertex_faces: HashMap<u32, Vec<usize>>,
}

impl MeshSimplifier {
    /// Create new mesh simplifier from vertices and indices
    pub fn new(vertices: &[Vertex], indices: &[u32]) -> Self {
        let mut positions = Vec::with_capacity(vertices.len());
        let mut vertex_quadrics = vec![Quadric::new(); vertices.len()];
        let mut faces = Vec::new();
        let mut edges = HashSet::new();
        let mut vertex_faces: HashMap<u32, Vec<usize>> = HashMap::new();
        
        // Extract positions
        for vertex in vertices {
            positions.push(Vector3::from(vertex.position));
        }
        
        // Build face list and compute initial quadrics
        for (face_idx, chunk) in indices.chunks(3).enumerate() {
            if chunk.len() == 3 {
                let face = [chunk[0], chunk[1], chunk[2]];
                faces.push(face);
                
                // Add edges
                edges.insert(order_edge(face[0], face[1]));
                edges.insert(order_edge(face[1], face[2]));
                edges.insert(order_edge(face[2], face[0]));
                
                // Track vertex-face relationships
                for &v in &face {
                    vertex_faces.entry(v).or_insert_with(Vec::new).push(face_idx);
                }
                
                // Compute face plane
                let v0 = positions[face[0] as usize];
                let v1 = positions[face[1] as usize];
                let v2 = positions[face[2] as usize];
                
                let normal = (v1 - v0).cross(v2 - v0).normalize();
                let d = -normal.dot(v0);
                
                let face_quadric = Quadric::from_plane(normal.x, normal.y, normal.z, d);
                
                // Add to vertex quadrics
                for &v in &face {
                    if let Some(quadric) = vertex_quadrics.get_mut(v as usize) {
                        *quadric = quadric.add(&face_quadric);
                    }
                }
            }
        }
        
        Self {
            vertex_quadrics,
            faces,
            positions,
            edges,
            vertex_faces,
        }
    }
    
    /// Simplify mesh to target triangle count
    pub fn simplify(&mut self, target_triangles: usize) -> SimplifiedMesh {
        let mut collapse_queue = BinaryHeap::new();
        let mut vertex_map: HashMap<u32, u32> = HashMap::new();
        let mut removed_vertices = HashSet::new();
        let mut removed_faces = HashSet::new();
        
        // Initialize vertex mapping
        for i in 0..self.positions.len() {
            vertex_map.insert(i as u32, i as u32);
        }
        
        // Build initial collapse candidates
        for &edge in &self.edges {
            if let Some(candidate) = self.compute_collapse_candidate(edge) {
                collapse_queue.push(candidate);
            }
        }
        
        // Perform edge collapses
        let mut current_triangles = self.faces.len();
        
        while current_triangles > target_triangles && !collapse_queue.is_empty() {
            let candidate = match collapse_queue.pop() {
                Some(c) => c,
                None => break, // Should not happen due to is_empty check, but be safe
            };
            let (v0, v1) = candidate.edge;
            
            // Skip if vertices already collapsed
            if removed_vertices.contains(&v0) || removed_vertices.contains(&v1) {
                continue;
            }
            
            // Perform collapse: v1 -> v0
            removed_vertices.insert(v1);
            vertex_map.insert(v1, v0);
            
            // Update position to optimal
            if let Some(pos) = self.positions.get_mut(v0 as usize) {
                *pos = candidate.target_position;
            }
            
            // Update quadric
            if let (Some(q0), Some(q1)) = (self.vertex_quadrics.get_mut(v0 as usize), self.vertex_quadrics.get(v1 as usize)) {
                *q0 = q0.add(q1);
            }
            
            // Remove degenerate faces and update topology
            if let Some(faces) = self.vertex_faces.get(&v1).cloned() {
                for face_idx in faces {
                    if !removed_faces.contains(&face_idx) {
                        let face = &mut self.faces[face_idx];
                        
                        // Replace v1 with v0
                        for v in face.iter_mut() {
                            if *v == v1 {
                                *v = v0;
                            }
                        }
                        
                        // Check for degenerate face
                        if face[0] == face[1] || face[1] == face[2] || face[2] == face[0] {
                            removed_faces.insert(face_idx);
                            current_triangles -= 1;
                        } else {
                            // Update vertex-face mapping
                            self.vertex_faces.entry(v0).or_insert_with(Vec::new).push(face_idx);
                        }
                    }
                }
            }
            
            // Update edges and recompute candidates for affected vertices
            let affected_vertices = self.get_connected_vertices(v0);
            for &v in &affected_vertices {
                if !removed_vertices.contains(&v) {
                    let edge = order_edge(v0, v);
                    if let Some(candidate) = self.compute_collapse_candidate(edge) {
                        collapse_queue.push(candidate);
                    }
                }
            }
        }
        
        // Build simplified mesh
        self.build_simplified_mesh(&vertex_map, &removed_vertices, &removed_faces)
    }
    
    /// Compute collapse candidate for edge
    fn compute_collapse_candidate(&self, edge: (u32, u32)) -> Option<CollapseCandidate> {
        let (v0, v1) = edge;
        
        if v0 >= self.positions.len() as u32 || v1 >= self.positions.len() as u32 {
            return None;
        }
        
        // Compute combined quadric
        let q0 = self.vertex_quadrics.get(v0 as usize)?;
        let q1 = self.vertex_quadrics.get(v1 as usize)?;
        let q_combined = q0.add(q1);
        
        // Find optimal position (simplified: use midpoint)
        let pos0 = self.positions.get(v0 as usize)?;
        let pos1 = self.positions.get(v1 as usize)?;
        let target_position = (pos0 + pos1) * 0.5;
        
        // Compute error
        let error = q_combined.compute_error(target_position);
        
        Some(CollapseCandidate {
            edge,
            error,
            target_position,
        })
    }
    
    /// Get vertices connected to given vertex
    fn get_connected_vertices(&self, vertex: u32) -> Vec<u32> {
        let mut connected = Vec::new();
        
        if let Some(faces) = self.vertex_faces.get(&vertex) {
            for &face_idx in faces {
                let face = self.faces.get(face_idx).copied()?;
                for &v in &face {
                    if v != vertex {
                        connected.push(v);
                    }
                }
            }
        }
        
        connected.sort_unstable();
        connected.dedup();
        connected
    }
    
    /// Build final simplified mesh
    fn build_simplified_mesh(
        &self,
        vertex_map: &HashMap<u32, u32>,
        removed_vertices: &HashSet<u32>,
        removed_faces: &HashSet<usize>,
    ) -> SimplifiedMesh {
        let mut new_vertices = Vec::new();
        let mut new_indices = Vec::new();
        let mut vertex_remap = HashMap::new();
        let mut next_index = 0u32;
        
        // Build new vertex list
        for (old_idx, pos) in self.positions.iter().enumerate() {
            if !removed_vertices.contains(&(old_idx as u32)) {
                new_vertices.push(Vertex {
                    position: (*pos).into(),
                    normal: [0.0, 1.0, 0.0], // Will be recomputed
                    tex_coords: [0.0, 0.0], // Simplified for now
                    color: [1.0, 1.0, 1.0, 1.0],
                    ao: 1.0,
                });
                vertex_remap.insert(old_idx as u32, next_index);
                next_index += 1;
            }
        }
        
        // Build new index list
        for (face_idx, face) in self.faces.iter().enumerate() {
            if !removed_faces.contains(&face_idx) {
                let mut new_face = [0u32; 3];
                let mut valid = true;
                
                for (i, &v) in face.iter().enumerate() {
                    let mapped_v = follow_vertex_map(vertex_map, v);
                    if let Some(&new_v) = vertex_remap.get(&mapped_v) {
                        new_face[i] = new_v;
                    } else {
                        valid = false;
                        break;
                    }
                }
                
                if valid && new_face[0] != new_face[1] && 
                   new_face[1] != new_face[2] && new_face[2] != new_face[0] {
                    new_indices.extend_from_slice(&new_face);
                }
            }
        }
        
        // Recompute normals
        recompute_normals(&mut new_vertices, &new_indices);
        
        SimplifiedMesh {
            vertices: new_vertices,
            indices: new_indices,
            reduction_ratio: 1.0 - (new_indices.len() as f32 / (self.faces.len() * 3) as f32),
        }
    }
}

/// Simplified mesh result
pub struct SimplifiedMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub reduction_ratio: f32,
}

/// Order edge vertices consistently
fn order_edge(v0: u32, v1: u32) -> (u32, u32) {
    if v0 < v1 { (v0, v1) } else { (v1, v0) }
}

/// Follow vertex mapping chain
fn follow_vertex_map(vertex_map: &HashMap<u32, u32>, mut vertex: u32) -> u32 {
    let mut visited = HashSet::new();
    
    while let Some(&mapped) = vertex_map.get(&vertex) {
        if mapped == vertex || visited.contains(&vertex) {
            break;
        }
        visited.insert(vertex);
        vertex = mapped;
    }
    
    vertex
}

/// Recompute vertex normals from faces
fn recompute_normals(vertices: &mut [Vertex], indices: &[u32]) {
    // Zero all normals
    for vertex in vertices.iter_mut() {
        vertex.normal = [0.0, 0.0, 0.0];
    }
    
    // Accumulate face normals
    for chunk in indices.chunks(3) {
        if chunk.len() == 3 {
            let v0 = Vector3::from(vertices[chunk[0] as usize].position);
            let v1 = Vector3::from(vertices[chunk[1] as usize].position);
            let v2 = Vector3::from(vertices[chunk[2] as usize].position);
            
            let normal = (v1 - v0).cross(v2 - v0);
            
            for &idx in chunk {
                let vertex = &mut vertices[idx as usize];
                let current = Vector3::from(vertex.normal);
                let new_normal = current + normal;
                vertex.normal = new_normal.into();
            }
        }
    }
    
    // Normalize
    for vertex in vertices.iter_mut() {
        let normal = Vector3::from(vertex.normal);
        if normal.magnitude() > 0.0 {
            vertex.normal = normal.normalize().into();
        }
    }
}