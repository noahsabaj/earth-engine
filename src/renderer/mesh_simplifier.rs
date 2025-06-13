/// Data-Oriented Mesh Simplification System
/// 
/// Sprint 37: Converted from OOP to pure functions operating on data structures.
/// Reduces mesh complexity for distant chunks using quadric error metrics.
/// Preserves visual quality while dramatically reducing triangle count.

use cgmath::{Vector3, Matrix4, Zero};
use crate::renderer::Vertex;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;

/// Quadric error matrix for vertex - pure data
#[derive(Debug, Clone, Copy)]
pub struct Quadric {
    pub matrix: Matrix4<f32>,
}

/// Pure functions for Quadric operations
/// No methods - just data transformations following DOP principles

/// Create empty quadric
pub fn create_quadric() -> Quadric {
    Quadric {
        matrix: Matrix4::zero(),
    }
}

/// Create quadric from plane equation
/// Pure function - transforms plane coefficients into quadric matrix
pub fn quadric_from_plane(a: f32, b: f32, c: f32, d: f32) -> Quadric {
    let matrix = Matrix4::new(
        a * a, a * b, a * c, a * d,
        a * b, b * b, b * c, b * d,
        a * c, b * c, c * c, c * d,
        a * d, b * d, c * d, d * d,
    );
    Quadric { matrix }
}

/// Add two quadrics
/// Pure function - combines quadric data
pub fn add_quadrics(q1: &Quadric, q2: &Quadric) -> Quadric {
    Quadric {
        matrix: q1.matrix + q2.matrix,
    }
}

/// Compute error for vertex position
/// Pure function - calculates quadric error at given position
pub fn compute_quadric_error(quadric: &Quadric, pos: Vector3<f32>) -> f32 {
    let v = cgmath::Vector4::new(pos.x, pos.y, pos.z, 1.0);
    let result = quadric.matrix * v;
    v.dot(result).abs()
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
        match self.partial_cmp(other) {
            Some(ordering) => ordering,
            None => {
                log::warn!("Invalid error values during collapse candidate comparison");
                Ordering::Equal
            }
        }
    }
}

/// Mesh simplification data structure (no methods)
/// Pure data - manipulated by free functions only
pub struct MeshSimplifierData {
    /// Vertex quadrics
    pub vertex_quadrics: Vec<Quadric>,
    
    /// Face list for topology
    pub faces: Vec<[u32; 3]>,
    
    /// Vertex positions
    pub positions: Vec<Vector3<f32>>,
    
    /// Valid edges
    pub edges: HashSet<(u32, u32)>,
    
    /// Vertex to faces mapping
    pub vertex_faces: HashMap<u32, Vec<usize>>,
}

/// Create mesh simplifier data from vertices and indices
/// Pure function - transforms vertex/index data into simplifier data structure
pub fn create_mesh_simplifier_data(vertices: &[Vertex], indices: &[u32]) -> MeshSimplifierData {
    let mut positions = Vec::with_capacity(vertices.len());
    let mut vertex_quadrics = vec![create_quadric(); vertices.len()];
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
            let v0 = match positions.get(face[0] as usize) {
                Some(&pos) => pos,
                None => {
                    log::warn!("Vertex {} out of bounds during face plane computation", face[0]);
                    Vector3::zero()
                }
            };
            let v1 = match positions.get(face[1] as usize) {
                Some(&pos) => pos,
                None => {
                    log::warn!("Vertex {} out of bounds during face plane computation", face[1]);
                    Vector3::zero()
                }
            };
            let v2 = match positions.get(face[2] as usize) {
                Some(&pos) => pos,
                None => {
                    log::warn!("Vertex {} out of bounds during face plane computation", face[2]);
                    Vector3::zero()
                }
            };
            
            let normal = (v1 - v0).cross(v2 - v0).normalize();
            let d = -normal.dot(v0);
            
            let face_quadric = quadric_from_plane(normal.x, normal.y, normal.z, d);
            
            // Add to vertex quadrics
            for &v in &face {
                if let Some(quadric) = vertex_quadrics.get_mut(v as usize) {
                    *quadric = add_quadrics(quadric, &face_quadric);
                }
            }
        }
    }
    
    MeshSimplifierData {
        vertex_quadrics,
        faces,
        positions,
        edges,
        vertex_faces,
    }
}

/// Simplified mesh result
#[derive(Debug)]
pub struct SimplifiedMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

/// Simplify mesh to target triangle count  
/// Pure function - transforms simplifier data to create simplified mesh
pub fn simplify_mesh(data: &mut MeshSimplifierData, target_triangles: usize) -> SimplifiedMesh {
    // For now, return the original mesh data without simplification
    // TODO: Implement the full quadric error metric simplification algorithm
    let vertices: Vec<Vertex> = data.positions.iter().map(|pos| {
        Vertex {
            position: [pos.x, pos.y, pos.z],
            normal: [0.0, 1.0, 0.0],
            tex_coords: [0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }).collect();
    
    let indices: Vec<u32> = data.faces.iter().flat_map(|face| face.iter()).cloned().collect();
    
    SimplifiedMesh { vertices, indices }
}

/// Helper function to order edges consistently
fn order_edge(a: u32, b: u32) -> (u32, u32) {
    if a < b { (a, b) } else { (b, a) }
}

// ===== COMPATIBILITY LAYER =====
// Temporary wrapper to maintain compatibility with existing code

#[deprecated(note = "Use MeshSimplifierData and pure functions instead")]
pub type MeshSimplifier = MeshSimplifierData;

impl MeshSimplifierData {
    /// Compatibility wrapper - use create_mesh_simplifier_data instead
    #[deprecated(note = "Use create_mesh_simplifier_data function instead")]
    pub fn new(vertices: &[Vertex], indices: &[u32]) -> Self {
        create_mesh_simplifier_data(vertices, indices)
    }
    
    /// Compatibility wrapper - use simplify_mesh function instead  
    #[deprecated(note = "Use simplify_mesh function instead")]
    pub fn simplify(&mut self, target_triangles: usize) -> SimplifiedMesh {
        simplify_mesh(self, target_triangles)
    }
}