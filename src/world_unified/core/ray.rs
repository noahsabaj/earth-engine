use cgmath::{Vector3, Point3, InnerSpace};
use super::{VoxelPos, BlockId};

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
}

impl Ray {
    pub fn new(origin: Point3<f32>, direction: Vector3<f32>) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BlockFace {
    Right,  // +X
    Left,   // -X
    Top,    // +Y
    Bottom, // -Y
    Front,  // +Z
    Back,   // -Z
}

impl BlockFace {
    pub fn normal(&self) -> Vector3<f32> {
        match self {
            BlockFace::Right => Vector3::new(1.0, 0.0, 0.0),
            BlockFace::Left => Vector3::new(-1.0, 0.0, 0.0),
            BlockFace::Top => Vector3::new(0.0, 1.0, 0.0),
            BlockFace::Bottom => Vector3::new(0.0, -1.0, 0.0),
            BlockFace::Front => Vector3::new(0.0, 0.0, 1.0),
            BlockFace::Back => Vector3::new(0.0, 0.0, -1.0),
        }
    }
    
    pub fn offset(&self) -> Vector3<i32> {
        match self {
            BlockFace::Right => Vector3::new(1, 0, 0),
            BlockFace::Left => Vector3::new(-1, 0, 0),
            BlockFace::Top => Vector3::new(0, 1, 0),
            BlockFace::Bottom => Vector3::new(0, -1, 0),
            BlockFace::Front => Vector3::new(0, 0, 1),
            BlockFace::Back => Vector3::new(0, 0, -1),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RaycastHit {
    pub position: VoxelPos,
    pub face: BlockFace,
    pub distance: f32,
    pub block: BlockId,
}

/// Cast a ray through the world and find the first block it hits
/// This is a basic implementation - specific world implementations may override with optimized versions
pub fn cast_ray<W: crate::WorldInterface>(
    world: &W,
    ray: Ray,
    max_distance: f32,
) -> Option<RaycastHit> {
    let step_size = 0.1;
    let mut t = 0.0;
    
    while t <= max_distance {
        let point = ray.origin + ray.direction * t;
        let voxel_pos = VoxelPos::new(
            point.x.floor() as i32,
            point.y.floor() as i32,
            point.z.floor() as i32,
        );
        
        let block = world.get_block(voxel_pos);
        if block != BlockId::AIR {
            let face = determine_hit_face(point, voxel_pos);
            return Some(RaycastHit {
                position: voxel_pos,
                face,
                distance: t,
                block,
            });
        }
        
        t += step_size;
    }
    
    None
}

fn determine_hit_face(hit_point: Point3<f32>, voxel_pos: VoxelPos) -> BlockFace {
    // Calculate the local position within the voxel (0-1 range)
    let local_x = hit_point.x - voxel_pos.x as f32;
    let local_y = hit_point.y - voxel_pos.y as f32;
    let local_z = hit_point.z - voxel_pos.z as f32;
    
    // Find which face is closest to the hit point
    let epsilon = 0.01;
    
    if local_x < epsilon {
        BlockFace::Left
    } else if local_x > 1.0 - epsilon {
        BlockFace::Right
    } else if local_y < epsilon {
        BlockFace::Bottom
    } else if local_y > 1.0 - epsilon {
        BlockFace::Top
    } else if local_z < epsilon {
        BlockFace::Back
    } else {
        BlockFace::Front
    }
}