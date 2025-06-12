/// Placeholder for greedy mesher module
pub struct GreedyMesher;
pub struct GreedyMeshStats;
pub struct GreedyQuad;

#[derive(Debug, Clone, Copy)]
pub enum FaceDirection {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

impl GreedyMesher {
    pub fn new() -> Self {
        Self
    }
}