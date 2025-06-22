//! Tests for GPU buffer layouts

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::mem;

    #[test]
    fn test_buffer_sizes() {
        // Verify all buffer sizes are correct
        assert_eq!(mem::size_of::<VoxelData>(), 4);
        assert_eq!(mem::size_of::<ChunkMetadata>(), 16);
        assert_eq!(mem::size_of::<InstanceData>(), 96);
        assert_eq!(mem::size_of::<CullingInstanceData>(), 32);
        assert_eq!(mem::size_of::<IndirectDrawCommand>(), 16);
        assert_eq!(mem::size_of::<IndirectDrawIndexedCommand>(), 20);
        assert_eq!(mem::size_of::<DrawMetadata>(), 32);
        assert_eq!(mem::size_of::<CameraUniform>(), 256);
        assert_eq!(mem::size_of::<CullingCameraData>(), 256);

        // Verify constants match
        assert_eq!(VOXEL_DATA_SIZE, 4);
        assert_eq!(CHUNK_METADATA_SIZE, 16);
        assert_eq!(INSTANCE_DATA_SIZE, 96);
        assert_eq!(CULLING_INSTANCE_SIZE, 32);
        assert_eq!(INDIRECT_COMMAND_SIZE, 16);
        assert_eq!(INDIRECT_INDEXED_COMMAND_SIZE, 20);
        assert_eq!(DRAW_METADATA_SIZE, 32);
        assert_eq!(CAMERA_UNIFORM_SIZE, 256);
        assert_eq!(CULLING_CAMERA_SIZE, 256);
    }

    #[test]
    fn test_voxel_data_packing() {
        let voxel = VoxelData::new(12345, 15, 10, 7);

        assert_eq!(voxel.block_id(), 12345);
        assert_eq!(voxel.light_level(), 15);
        assert_eq!(voxel.sky_light_level(), 10);
        assert_eq!(voxel.metadata(), 7);

        // Test air block
        assert!(VoxelData::AIR.is_air());
        assert_eq!(VoxelData::AIR.block_id(), 0);

        // Test mutations
        let voxel2 = voxel.with_block_id(100);
        assert_eq!(voxel2.block_id(), 100);
        assert_eq!(voxel2.light_level(), 15); // Unchanged

        let voxel3 = voxel.with_light_level(5);
        assert_eq!(voxel3.block_id(), 12345); // Unchanged
        assert_eq!(voxel3.light_level(), 5);
    }

    #[test]
    fn test_chunk_calculations() {
        let layout = WorldBufferLayout::new(3);

        assert_eq!(layout.view_distance, 3);
        assert_eq!(layout.max_chunks, 343); // 7³

        // Test slot offset calculation
        assert_eq!(layout.chunk_offset(0), 0);
        assert_eq!(layout.chunk_offset(1), CHUNK_BUFFER_SLOT_SIZE);
        assert_eq!(layout.chunk_offset(10), 10 * CHUNK_BUFFER_SLOT_SIZE);

        // Test memory calculation
        let memory_mb = layout.memory_usage_mb();
        assert!(memory_mb > 40.0 && memory_mb < 50.0); // ~45 MB expected
    }

    #[test]
    fn test_instance_data() {
        use cgmath::{Matrix4, Vector3};

        let instance = InstanceData::new(Vector3::new(10.0, 20.0, 30.0), 2.0, [1.0, 0.5, 0.0, 1.0]);

        let pos = instance.position();
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);
        assert_eq!(pos.z, 30.0);

        // Test culling instance conversion
        let culling = CullingInstanceData::from_instance(&instance, 5.0, 42);
        assert_eq!(culling.position, [10.0, 20.0, 30.0]);
        assert_eq!(culling.radius, 5.0);
        assert_eq!(culling.instance_id, 42);
        assert!(culling.is_visible());
        assert!(culling.casts_shadows());
    }

    #[test]
    fn test_buffer_alignment() {
        // Test alignment helper
        assert_eq!(calculations::align_buffer_size(100, 16), 112);
        assert_eq!(calculations::align_buffer_size(128, 16), 128);
        assert_eq!(calculations::align_buffer_size(129, 16), 144);

        assert_eq!(calculations::align_buffer_size(100, 256), 256);
        assert_eq!(calculations::align_buffer_size(256, 256), 256);
        assert_eq!(calculations::align_buffer_size(257, 256), 512);
    }

    #[test]
    fn test_memory_budget() {
        // Test chunk calculations for memory budgets
        let chunks_128mb = chunks_per_memory_budget(128);
        let chunks_512mb = chunks_per_memory_budget(512);

        assert!(chunks_128mb < chunks_512mb);
        assert!(chunks_128mb > 0);

        // Test view distance recommendations
        assert_eq!(recommended_view_distance(64), 2);
        assert_eq!(recommended_view_distance(256), 3);
        assert_eq!(recommended_view_distance(512), 4);
        assert_eq!(recommended_view_distance(2048), 6);
    }

    #[test]
    fn test_indirect_commands() {
        let cmd = IndirectDrawCommand::new(100, 50);
        assert_eq!(cmd.vertex_count, 100);
        assert_eq!(cmd.instance_count, 50);
        assert_eq!(cmd.first_vertex, 0);
        assert_eq!(cmd.first_instance, 0);

        let indexed = IndirectDrawIndexedCommand::with_offsets(300, 10, 100, -5, 20);
        assert_eq!(indexed.index_count, 300);
        assert_eq!(indexed.instance_count, 10);
        assert_eq!(indexed.first_index, 100);
        assert_eq!(indexed.base_vertex, -5);
        assert_eq!(indexed.first_instance, 20);
    }

    #[test]
    fn test_draw_metadata() {
        let meta = DrawMetadata::new([10.0, 20.0, 30.0], 5.0, 42, 100);

        assert_eq!(meta.bounding_sphere, [10.0, 20.0, 30.0, 5.0]);
        assert_eq!(meta.material_id, 42);
        assert_eq!(meta.mesh_id, 100);
        assert!(meta.is_visible());
        assert!(meta.casts_shadows());
        assert!(!meta.is_transparent());

        let meta_lod = meta.with_lod_range(10.0, 100.0, 2);
        assert_eq!(meta_lod.lod_info[0], 10.0);
        assert_eq!(meta_lod.lod_info[1], 100.0);
        assert_eq!(meta_lod.lod_info[2], 2.0);
    }

    #[test]
    fn test_compute_dispatch_params() {
        use compute::{workgroup_sizes, ComputeDispatchParams};

        let dispatch_1d = ComputeDispatchParams::calculate_1d(1000, workgroup_sizes::MEDIUM);
        assert_eq!(dispatch_1d.total_items, 1000);
        assert_eq!(dispatch_1d.items_per_workgroup, 128);
        assert_eq!(dispatch_1d.workgroup_count[0], 8); // ceil(1000/128)
        assert_eq!(dispatch_1d.workgroup_count[1], 1);
        assert_eq!(dispatch_1d.workgroup_count[2], 1);

        let dispatch_2d = ComputeDispatchParams::calculate_2d(256, 256, workgroup_sizes::TILE_2D);
        assert_eq!(dispatch_2d.total_items, 65536);
        assert_eq!(dispatch_2d.workgroup_count[0], 16);
        assert_eq!(dispatch_2d.workgroup_count[1], 16);
        assert_eq!(dispatch_2d.workgroup_count[2], 1);
    }

    #[test]
    fn test_vertex_soa() {
        use mesh::{Vertex, VertexSOA};

        let mut soa = VertexSOA::new();
        assert!(soa.is_empty());

        soa.push_vertex(&Vertex::new([1.0, 2.0, 3.0], [0.0, 1.0, 0.0], [0.5, 0.5]));

        assert_eq!(soa.len(), 1);
        assert_eq!(soa.positions_x[0], 1.0);
        assert_eq!(soa.positions_y[0], 2.0);
        assert_eq!(soa.positions_z[0], 3.0);
        assert_eq!(soa.normals_y[0], 1.0);
        assert_eq!(soa.tex_coords_u[0], 0.5);
    }

    #[test]
    fn test_terrain_params_conversion() {
        let aos = TerrainParams::default();
        let soa = TerrainParamsSOA::from_aos(&aos);
        let aos2 = soa.to_aos();

        // Verify round-trip conversion
        assert_eq!(aos.seed, aos2.seed);
        assert_eq!(aos.sea_level, aos2.sea_level);
        assert_eq!(aos.distribution_count, aos2.distribution_count);

        for i in 0..aos.distribution_count as usize {
            assert_eq!(
                aos.distributions[i].block_id,
                aos2.distributions[i].block_id
            );
            assert_eq!(aos.distributions[i].min_y, aos2.distributions[i].min_y);
            assert_eq!(
                aos.distributions[i].threshold,
                aos2.distributions[i].threshold
            );
        }
    }
}
