#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluid::*;
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
    async fn test_fluid_voxel_packing() {
        // Test fluid voxel data packing
        let voxel = FluidVoxel::new(FluidType::Water, 0.75, 25.0);
        
        assert_eq!(voxel.get_type(), FluidType::Water);
        assert!((voxel.get_level() - 0.75).abs() < 0.01);
        assert!((voxel.get_temperature() - 25.0).abs() < 1.0);
    }
    
    #[tokio::test]
    async fn test_fluid_buffer_creation() {
        let (device, _queue) = create_test_context().await;
        let device = std::sync::Arc::new(device);
        
        let size = (64, 32, 64);
        let fluid_buffer = FluidBuffer::new(device.clone(), size);
        
        assert_eq!(fluid_buffer.size, size);
        assert!(fluid_buffer.current_buffer.is_some());
        assert!(fluid_buffer.previous_buffer.is_some());
    }
    
    #[tokio::test]
    async fn test_fluid_compute_pipeline() {
        let (device, queue) = create_test_context().await;
        let device = std::sync::Arc::new(device);
        
        let size = (32, 16, 32);
        let mut fluid_buffer = FluidBuffer::new(device.clone(), size);
        let fluid_compute = FluidCompute::new(device.clone());
        
        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Test Encoder"),
        });
        
        // Run one update step
        fluid_compute.update(&mut encoder, &mut fluid_buffer, 0.016);
        
        // Submit commands
        queue.submit(Some(encoder.finish()));
        
        // Verify no panics
        device.poll(wgpu::Maintain::Wait);
    }
    
    #[tokio::test]
    async fn test_pressure_solver() {
        let (device, queue) = create_test_context().await;
        let device = std::sync::Arc::new(device);
        
        let size = (16, 8, 16);
        let fluid_buffer = FluidBuffer::new(device.clone(), size);
        let pressure_solver = PressureSolver::new(device.clone());
        
        // Create test bind group
        let bind_group_layout = pressure_solver.get_bind_group_layout();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Test Bind Group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: fluid_buffer.current_buffer.as_ref().unwrap(),
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Test Encoder"),
        });
        
        // Run pressure solve
        pressure_solver.solve(&mut encoder, &fluid_buffer, &bind_group, 10);
        
        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);
    }
    
    #[tokio::test]
    async fn test_multi_phase_system() {
        let (device, _queue) = create_test_context().await;
        let device = std::sync::Arc::new(device);
        
        let phase_system = PhaseSystem::new(device.clone());
        
        // Test phase properties
        let props = PhaseProperties::default();
        
        // Water-Air interaction
        let water_air = props.interactions[FluidType::Water as usize][FluidType::Air as usize];
        assert_eq!(water_air.miscibility, 0.0); // Immiscible
        assert!((water_air.interface_tension - 0.072).abs() < 0.001);
        
        // Water-Lava interaction
        let water_lava = props.interactions[FluidType::Water as usize][FluidType::Lava as usize];
        assert_eq!(water_lava.heat_transfer, 10.0); // High heat transfer
    }
    
    #[tokio::test]
    async fn test_terrain_interaction() {
        let (device, queue) = create_test_context().await;
        let device = std::sync::Arc::new(device);
        
        let mut terrain_interaction = TerrainInteraction::new(device.clone());
        
        // Initialize sediment buffer
        terrain_interaction.init_sediment_buffer((32, 16, 32));
        
        // Update erosion parameters
        let erosion_params = ErosionParams {
            water_erosion_rate: 0.02,
            lava_erosion_rate: 0.002,
            sediment_capacity: 0.15,
            deposition_rate: 0.025,
            erosion_threshold: 0.6,
            evaporation_rate: 0.0002,
            _padding: [0.0; 2],
        };
        
        terrain_interaction.update_erosion_params(&queue, &erosion_params);
    }
    
    #[tokio::test]
    async fn test_fluid_renderer() {
        let (device, queue) = create_test_context().await;
        let device = std::sync::Arc::new(device);
        
        let output_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let fluid_renderer = FluidRenderer::new(device.clone(), output_format);
        
        // Update render parameters
        let render_params = FluidRenderParams {
            water_refraction: 1.4,
            water_opacity: 0.85,
            lava_opacity: 1.0,
            oil_opacity: 0.95,
            smoothing_factor: 0.6,
            foam_threshold: 2.5,
            reflection_strength: 0.4,
            _padding: 0.0,
        };
        
        fluid_renderer.update_render_params(&queue, &render_params);
    }
    
    #[tokio::test]
    async fn test_performance_monitor() {
        let mut monitor = FluidPerformanceMonitor::new();
        
        // Simulate frame
        monitor.begin_frame();
        
        // Record timings
        monitor.record_update_time(std::time::Duration::from_millis(5));
        monitor.record_solver_time(std::time::Duration::from_millis(3));
        monitor.record_render_time(std::time::Duration::from_millis(2));
        
        // Update stats
        monitor.set_active_voxels(50000);
        monitor.set_memory_usage(100 * 1024 * 1024); // 100 MB
        
        // Check metrics
        let metrics = monitor.get_metrics();
        assert!(metrics.memory_usage_mb > 0.0);
        
        // Check performance status
        let status = monitor.check_performance();
        assert_eq!(status, PerformanceStatus::Good);
    }
    
    #[test]
    fn test_fluid_constants() {
        let constants = FluidConstants::water();
        assert!((constants.density - 1000.0).abs() < 0.1);
        assert!((constants.viscosity - 0.001).abs() < 0.0001);
        
        let lava_constants = FluidConstants::lava();
        assert!(lava_constants.density > constants.density);
        assert!(lava_constants.viscosity > constants.viscosity);
    }
    
    #[test]
    fn test_fluid_reactions() {
        // Test water-lava reaction
        let reaction = FluidReaction::check_reaction(FluidType::Water, FluidType::Lava);
        assert!(matches!(reaction, Some(FluidReaction::WaterLava)));
        
        // Test symmetric reaction
        let reaction2 = FluidReaction::check_reaction(FluidType::Lava, FluidType::Water);
        assert!(matches!(reaction2, Some(FluidReaction::WaterLava)));
        
        // Test no reaction
        let no_reaction = FluidReaction::check_reaction(FluidType::Water, FluidType::Air);
        assert!(no_reaction.is_none());
    }
}