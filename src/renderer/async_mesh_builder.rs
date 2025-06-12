/// Placeholder for async mesh builder module
use crate::world::ChunkPos;

pub struct AsyncMeshBuilder;
pub struct MeshBuildRequest {
    pub chunk_pos: ChunkPos,
}
pub struct CompletedMesh;
pub struct MeshBuildStats;

impl AsyncMeshBuilder {
    pub fn new() -> Self {
        Self
    }
}