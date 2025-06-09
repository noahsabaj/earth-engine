use earth_engine::renderer::gpu_driven::*;
use std::sync::Arc;
use cgmath::Vector3;

#[tokio::test]
async fn test_indirect_command_buffer() {
    // Setup GPU
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .unwrap();
    
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    
    // Create command buffer
    let mut command_buffer = IndirectCommandBuffer::new(&device, 100, true);
    
    // Test command update
    let commands = vec![
        IndirectDrawIndexedCommand {
            index_count: 36,
            instance_count: 1,
            first_index: 0,
            base_vertex: 0,
            first_instance: 0,
        },
        IndirectDrawIndexedCommand {
            index_count: 36,
            instance_count: 1,
            first_index: 36,
            base_vertex: 0,
            first_instance: 1,
        },
    ];
    
    command_buffer.update_indexed_commands(&queue, &commands);
    assert_eq!(command_buffer.count(), 2);
    
    // Test GPU copy
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    command_buffer.copy_to_gpu(&mut encoder);
    queue.submit(Some(encoder.finish()));
}

#[tokio::test]
async fn test_instance_buffer() {
    // Setup GPU
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .unwrap();
    
    // Create instance buffer
    let mut instance_buffer = InstanceBuffer::new(&device, 100);
    
    // Add instances
    let instance1 = InstanceData::new(
        Vector3::new(0.0, 0.0, 0.0),
        1.0,
        [1.0, 0.0, 0.0, 1.0],
    );
    let instance2 = InstanceData::new(
        Vector3::new(10.0, 0.0, 0.0),
        2.0,
        [0.0, 1.0, 0.0, 1.0],
    );
    
    let id1 = instance_buffer.add_instance(instance1).unwrap();
    let id2 = instance_buffer.add_instance(instance2).unwrap();
    
    assert_eq!(id1, 0);
    assert_eq!(id2, 1);
    assert_eq!(instance_buffer.count(), 2);
    
    // Update instance
    let updated = InstanceData::new(
        Vector3::new(5.0, 5.0, 5.0),
        1.5,
        [0.0, 0.0, 1.0, 1.0],
    );
    instance_buffer.update_instance(id1, updated);
    
    // Remove instance (swap-remove)
    let moved_id = instance_buffer.remove_instance(id1);
    assert_eq!(moved_id, Some(1)); // Instance 1 was moved to position 0
    assert_eq!(instance_buffer.count(), 1);
    
    // Upload to GPU
    instance_buffer.upload_to_gpu(&queue);
}

#[test]
fn test_lod_system() {
    let mut lod_system = LodSystem::new();
    
    // Create chunk LOD config
    let chunk_config = LodSystem::create_chunk_lod_config();
    lod_system.register_config(0, chunk_config);
    
    // Test LOD selection at various distances
    let camera_pos = Vector3::new(0.0, 50.0, 0.0);
    let screen_height = 1080.0;
    let fov_y = 45.0_f32.to_radians();
    
    // Close object - should select LOD 0
    let close_pos = Vector3::new(20.0, 50.0, 0.0);
    let lod = lod_system.select_lod(0, close_pos, camera_pos, screen_height, fov_y);
    assert!(lod.is_some());
    assert_eq!(lod.unwrap().level, 0);
    
    // Medium distance - should select LOD 1
    let medium_pos = Vector3::new(100.0, 50.0, 0.0);
    let lod = lod_system.select_lod(0, medium_pos, camera_pos, screen_height, fov_y);
    assert!(lod.is_some());
    assert_eq!(lod.unwrap().level, 1);
    
    // Far distance - should select LOD 2
    let far_pos = Vector3::new(300.0, 50.0, 0.0);
    let lod = lod_system.select_lod(0, far_pos, camera_pos, screen_height, fov_y);
    assert!(lod.is_some());
    assert_eq!(lod.unwrap().level, 2);
    
    // Very far - should select LOD 3
    let very_far_pos = Vector3::new(600.0, 50.0, 0.0);
    let lod = lod_system.select_lod(0, very_far_pos, camera_pos, screen_height, fov_y);
    assert!(lod.is_some());
    assert_eq!(lod.unwrap().level, 3);
    
    // Beyond max distance - should return None
    let beyond_pos = Vector3::new(2000.0, 50.0, 0.0);
    let lod = lod_system.select_lod(0, beyond_pos, camera_pos, screen_height, fov_y);
    assert!(lod.is_none());
}

#[test]
fn test_lod_bias() {
    let mut lod_system = LodSystem::new();
    
    // Register config
    let config = LodSystem::create_entity_lod_config(5.0);
    lod_system.register_config(1, config);
    
    let camera_pos = Vector3::new(0.0, 0.0, 0.0);
    let object_pos = Vector3::new(60.0, 0.0, 0.0);
    
    // Test with no bias
    lod_system.set_lod_bias(0.0);
    let lod1 = lod_system.select_lod(1, object_pos, camera_pos, 0.0, 0.0);
    
    // Test with positive bias (lower detail)
    lod_system.set_lod_bias(1.0);
    let lod2 = lod_system.select_lod(1, object_pos, camera_pos, 0.0, 0.0);
    
    // With positive bias, we should get a lower detail LOD
    if let (Some(l1), Some(l2)) = (lod1, lod2) {
        assert!(l2.level >= l1.level);
    }
}

#[test]
fn test_draw_metadata() {
    let metadata = DrawMetadata {
        bounding_sphere: [10.0, 20.0, 30.0, 5.0],
        lod_info: [0.0, 100.0, 200.0, 300.0],
        material_id: 1,
        mesh_id: 2,
        instance_offset: 42,
        flags: 0b0111, // Visible | Cast shadows | Always visible
    };
    
    // Test size for GPU alignment
    assert_eq!(std::mem::size_of::<DrawMetadata>(), 48);
    
    // Test that it's Pod (can be cast to bytes)
    let bytes = bytemuck::bytes_of(&metadata);
    assert_eq!(bytes.len(), 48);
}

#[test]
fn test_culling_instance_data() {
    let instance = InstanceData::new(
        Vector3::new(10.0, 20.0, 30.0),
        2.0,
        [1.0, 0.5, 0.0, 1.0],
    );
    
    let culling_data = CullingInstanceData::from_instance(&instance, 5.0, 123);
    
    assert_eq!(culling_data.position, [10.0, 20.0, 30.0]);
    assert_eq!(culling_data.radius, 5.0);
    assert_eq!(culling_data.instance_id, 123);
}

#[tokio::test]
async fn test_gpu_driven_renderer_creation() {
    // Setup GPU
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .unwrap();
    
    let device = Arc::new(device);
    let queue = Arc::new(queue);
    
    // Create camera bind group layout (dummy)
    let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Test Camera Layout"),
        entries: &[],
    });
    
    // Create renderer
    let renderer = GpuDrivenRenderer::new(
        device,
        queue,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        &camera_bind_group_layout,
    );
    
    // Check initial stats
    let stats = renderer.stats();
    assert_eq!(stats.objects_submitted, 0);
    assert_eq!(stats.objects_drawn, 0);
}

#[test]
fn test_render_object() {
    let obj = RenderObject {
        position: Vector3::new(1.0, 2.0, 3.0),
        scale: 2.5,
        color: [1.0, 0.0, 0.0, 1.0],
        bounding_radius: 5.0,
        mesh_id: 10,
        material_id: 20,
    };
    
    assert_eq!(obj.scale, 2.5);
    assert_eq!(obj.mesh_id, 10);
}