//! Pure data structures for progressive streaming - NO METHODS!
//! All operations are in renderer_operations.rs

use crate::renderer::renderer_data::{
    MeshPacketData, PacketType, ProgressiveMeshStateData,
    ProgressiveStreamerData, MeshUpdateData, VertexAttributesData,
    ProgressiveEncoderData
};

// Type aliases for clarity
pub type MeshPacket = MeshPacketData;
pub type ProgressiveMeshState = ProgressiveMeshStateData;
pub type ProgressiveStreamer = ProgressiveStreamerData;
pub type MeshUpdate = MeshUpdateData;
pub type VertexAttributes = VertexAttributesData;
pub type ProgressiveEncoder = ProgressiveEncoderData;

// Re-export packet types
pub use crate::renderer::renderer_data::PacketType as StreamPacketType;