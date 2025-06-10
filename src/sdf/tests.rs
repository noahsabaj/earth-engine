#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdf::*;
    use wgpu::util::DeviceExt;
    
    /// Create test device and queue
    async fn create_test_context() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("Failed to find adapter");
        
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Test Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");
        
        (device, queue)
    }
    
    #[tokio::test]
    async fn test_sdf_value_size() {
        // Ensure SdfValue is properly aligned and sized
        assert_eq!(std::mem::size_of::<SdfValue>(), 12);
        assert_eq!(std::mem::align_of::<SdfValue>(), 4);
    }
    
    #[tokio::test]
    async fn test_sdf_buffer_creation() {
        let (device, _queue) = create_test_context().await;
        let device = std::sync::Arc::new(device);
        
        let voxel_size = (32, 32, 32);
        let sdf_buffer = SdfBuffer::new(device.clone(), voxel_size);
        
        // Check size includes margins
        let expected_size = (
            (voxel_size.0 + 2 * SDF_MARGIN) * 2, // 2x resolution
            (voxel_size.1 + 2 * SDF_MARGIN) * 2,
            (voxel_size.2 + 2 * SDF_MARGIN) * 2,
        );
        assert_eq!(sdf_buffer.size, expected_size);
        assert!(sdf_buffer.buffer.is_some());
    }
    
    #[tokio::test]
    async fn test_sdf_generator() {
        let (device, queue) = create_test_context().await;
        let device = std::sync::Arc::new(device);
        
        let sdf_generator = SdfGenerator::new(device.clone());
        
        // Update constants
        let constants = SdfConstants::default();
        sdf_generator.update_constants(&queue, &constants);
    }
    
    #[tokio::test]
    async fn test_marching_cubes_table() {
        let (device, _queue) = create_test_context().await;
        
        let march_table = MarchTable::new(&device);
        
        // Tables should be created
        // In a real implementation, would verify table contents
    }
    
    #[tokio::test]
    async fn test_surface_extractor() {
        let (device, _queue) = create_test_context().await;
        let device = std::sync::Arc::new(device);
        
        let mut surface_extractor = SurfaceExtractor::new(device.clone());
        
        // Test extraction params
        let params = ExtractionParams::default();
        assert_eq!(params.threshold, 0.0);
        assert_eq!(params.smooth_iterations, 2);
    }
    
    #[tokio::test]
    async fn test_hybrid_collider() {
        let (device, _queue) = create_test_context().await;
        let device = std::sync::Arc::new(device);
        
        let mut collider = HybridCollider::new(device.clone());
        
        // Test mode switching
        collider.set_mode(CollisionMode::Voxel);
        assert_eq!(collider.mode, CollisionMode::Voxel);
        
        collider.set_mode(CollisionMode::Sdf);
        assert_eq!(collider.mode, CollisionMode::Sdf);
    }
    
    #[tokio::test]
    async fn test_lod_levels() {
        // Test LOD level properties
        assert_eq!(LodLevel::High.sdf_resolution(), 0.5);
        assert_eq!(LodLevel::Medium.sdf_resolution(), 1.0);
        assert_eq!(LodLevel::Low.sdf_resolution(), 2.0);
        
        assert_eq!(LodLevel::High.smoothing_iterations(), 1);
        assert_eq!(LodLevel::VeryLow.smoothing_iterations(), 4);
    }
    
    #[tokio::test]
    async fn test_lod_selection() {
        let (device, _queue) = create_test_context().await;
        let device = std::sync::Arc::new(device);
        
        let lod_system = SdfLod::new(device.clone());
        
        // Test LOD selection based on distance
        let chunk_pos = glam::Vec3::ZERO;
        let chunk_size = 32.0;
        
        // Close distance should select voxel rendering
        let close_camera = glam::Vec3::new(16.0, 16.0, 16.0);
        let close_lod = lod_system.select_lod(chunk_pos, chunk_size, close_camera, 0.0);
        assert_eq!(close_lod, LodLevel::Voxel);
        
        // Far distance should select low detail
        let far_camera = glam::Vec3::new(500.0, 500.0, 500.0);
        let far_lod = lod_system.select_lod(chunk_pos, chunk_size, far_camera, 0.0);
        assert!(matches!(far_lod, LodLevel::Low | LodLevel::VeryLow));
    }
    
    #[tokio::test]
    async fn test_dual_representation() {
        let (device, _queue) = create_test_context().await;
        let device = std::sync::Arc::new(device);
        
        // Create mock world buffer
        let world_buffer = Arc::new(crate::world_gpu::WorldBuffer::new(
            device.clone(),
            1024 * 1024 * 1024, // 1GB
        ));
        
        let mut dual_rep = DualRepresentation::new(device.clone(), world_buffer, 32);
        
        // Test render mode switching
        dual_rep.set_render_mode(RenderMode::Smooth);
        assert_eq!(dual_rep.render_mode, RenderMode::Smooth);
        
        // Test chunk marking
        let chunk_pos = glam::IVec3::new(0, 0, 0);
        dual_rep.mark_chunk_dirty(chunk_pos);
        assert!(dual_rep.dirty_chunks.contains(&chunk_pos));
        
        // Should also mark neighbors
        assert!(dual_rep.dirty_chunks.len() > 1);
    }
    
    #[test]
    fn test_sdf_constants() {
        let constants = SdfConstants::default();
        assert_eq!(constants.resolution_factor, 2.0);
        assert_eq!(constants.max_distance, SDF_MAX_DISTANCE);
        assert_eq!(constants.surface_threshold, SDF_SURFACE_THRESHOLD);
    }
    
    #[test]
    fn test_smooth_vertex_layout() {
        // Verify vertex struct layout for GPU compatibility
        assert_eq!(std::mem::size_of::<SmoothVertex>(), 48);
        assert_eq!(std::mem::align_of::<SmoothVertex>(), 4);
    }
    
    #[test]
    fn test_memory_stats() {
        let mut stats = MemoryStats::default();
        stats.voxel_memory = 1024 * 1024;
        stats.sdf_memory = 512 * 1024;
        stats.mesh_memory = 256 * 1024;
        stats.total_memory = stats.voxel_memory + stats.sdf_memory + stats.mesh_memory;
        
        assert_eq!(stats.total_memory, 1024 * 1024 + 512 * 1024 + 256 * 1024);
    }
    
    #[test]
    fn test_transition_settings() {
        let settings = TransitionSettings::default();
        assert_eq!(settings.start_distance, 50.0);
        assert_eq!(settings.end_distance, 100.0);
        assert_eq!(settings.blend_curve, 2.0);
    }
}