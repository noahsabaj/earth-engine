#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::ChunkPos;
    use crate::world_gpu::{
        WorldBuffer, WorldBufferDescriptor, VoxelData,
        TerrainGenerator, TerrainParams,
        ChunkModifier, ModificationCommand,
        GpuLighting,
        UnifiedMemoryManager, UnifiedMemoryLayout, SystemType, MemoryStats,
    };
    
    /// Helper to create a test GPU device
    async fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::empty(),
            dx12_shader_compiler: Default::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find adapter");
        
        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: Some("Test Device"),
                },
                None,
            )
            .await
            .expect("Failed to create device")
    }
    
    #[tokio::test]
    async fn test_world_buffer_creation() {
        let (device, _queue) = create_test_device().await;
        let device = std::sync::Arc::new(device);
        
        let desc = WorldBufferDescriptor {
            world_size: 16,
            enable_atomics: true,
            enable_readback: true,
        };
        
        let world_buffer = WorldBuffer::new(device.clone(), &desc);
        
        // Verify buffer sizes
        assert_eq!(world_buffer.world_size(), 16);
        assert_eq!(world_buffer.world_size(), 16);
        
        // Calculate expected sizes
        let total_chunks = 16 * 16 * 16; // 16x16x16 chunks (not 16x16x8!)
        let voxels_per_chunk = 32 * 32 * 32;
        let expected_voxel_size = total_chunks * voxels_per_chunk * 4; // 4 bytes per voxel
        let expected_metadata_size = total_chunks * 16; // 16 bytes per chunk
        
        assert_eq!(world_buffer.voxel_buffer().size(), expected_voxel_size as u64);
        assert_eq!(world_buffer.metadata_buffer().size(), expected_metadata_size as u64);
    }
    
    #[tokio::test]
    async fn test_voxel_data_packing() {
        // Test voxel data packing/unpacking
        let voxel = VoxelData::new(12345, 7, 15, 3);
        
        assert_eq!(voxel.block_id(), 12345);
        assert_eq!(voxel.light_level(), 7);
        assert_eq!(voxel.sky_light_level(), 15);
        assert_eq!(voxel.metadata(), 3);
        
        // Test with maximum values
        let max_voxel = VoxelData::new(65535, 15, 15, 255);
        assert_eq!(max_voxel.block_id(), 65535);
        assert_eq!(max_voxel.light_level(), 15);
        assert_eq!(max_voxel.sky_light_level(), 15);
        assert_eq!(max_voxel.metadata(), 255);
    }
    
    #[tokio::test]
    async fn test_terrain_generation() {
        let (device, queue) = create_test_device().await;
        let device = std::sync::Arc::new(device);
        
        // Create world buffer (small size for testing)
        let world_buffer = WorldBuffer::new(device.clone(), &WorldBufferDescriptor {
            world_size: 8,
            enable_atomics: true,
            enable_readback: true,
        });
        
        // Create terrain generator
        let terrain_gen = TerrainGenerator::new(device.clone());
        
        // Update parameters
        let params = TerrainParams {
            seed: 42,
            sea_level: 64.0,
            terrain_scale: 0.02,
            mountain_threshold: 0.7,
            cave_threshold: 0.3,
            ore_chances: [0.1, 0.05, 0.02, 0.01],
        };
        terrain_gen.update_params(&queue, &params);
        
        // Generate some chunks
        let chunks_to_generate = vec![
            ChunkPos { x: 0, y: 0, z: 0 },
            ChunkPos { x: 1, y: 0, z: 0 },
            ChunkPos { x: 0, y: 0, z: 1 },
            ChunkPos { x: 1, y: 0, z: 1 },
        ];
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Test Terrain Generation"),
        });
        
        terrain_gen.generate_chunks(&mut encoder, &world_buffer, &chunks_to_generate);
        
        queue.submit(std::iter::once(encoder.finish()));
        
        // Verify generation completed (would need to read back buffer in real test)
        // For now, just ensure no panic
    }
    
    #[tokio::test]
    async fn test_chunk_modification() {
        let (device, queue) = create_test_device().await;
        let device = std::sync::Arc::new(device);
        
        // Create world buffer (small size for testing)  
        let world_buffer = WorldBuffer::new(device.clone(), &WorldBufferDescriptor {
            world_size: 8,
            enable_atomics: true,
            enable_readback: true,
        });
        
        // Create chunk modifier
        let modifier = ChunkModifier::new(device.clone());
        
        // Test various modification commands
        let commands = vec![
            ModificationCommand::set_block(100, 64, 100, 1), // Place stone
            ModificationCommand::break_block(101, 64, 100),  // Break block
            ModificationCommand::explode(102, 64, 102, 5.0), // Explosion
        ];
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Test Modifications"),
        });
        
        modifier.apply_modifications(&mut encoder, &queue, &world_buffer, &commands);
        
        queue.submit(std::iter::once(encoder.finish()));
    }
    
    #[tokio::test]
    async fn test_ambient_occlusion() {
        let (device, queue) = create_test_device().await;
        let device = std::sync::Arc::new(device);
        
        // Create world buffer and lighting system (small size for testing)
        let world_buffer = WorldBuffer::new(device.clone(), &WorldBufferDescriptor {
            world_size: 8,
            enable_atomics: true,
            enable_readback: true,
        });
        
        let lighting = GpuLighting::new(device.clone());
        
        // Calculate AO for some chunks
        let chunks = vec![
            ChunkPos { x: 0, y: 0, z: 0 },
            ChunkPos { x: 1, y: 0, z: 0 },
        ];
        
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Test AO Calculation"),
        });
        
        lighting.calculate_ambient_occlusion(
            &mut encoder,
            &world_buffer,
            &chunks,
            2, // 2 smoothing passes
        );
        
        queue.submit(std::iter::once(encoder.finish()));
    }
    
    #[tokio::test]
    async fn test_unified_memory_layout() {
        let layout = UnifiedMemoryLayout::new(32, 64); // Use smaller test-safe values
        
        // Verify layout calculations
        assert_eq!(layout.world_size, 32);
        assert_eq!(layout.world_height, 64);
        assert_eq!(layout.chunk_size, 32);
        
        // Check offsets are properly aligned
        assert_eq!(layout.voxel_data_offset % 256, 0);
        assert_eq!(layout.chunk_metadata_offset % 256, 0);
        assert_eq!(layout.lighting_data_offset % 256, 0);
        assert_eq!(layout.entity_data_offset % 256, 0);
        assert_eq!(layout.particle_data_offset % 256, 0);
        
        // Verify chunk offset calculations
        let offset1 = layout.get_chunk_voxel_offset(0, 0, 0);
        let offset2 = layout.get_chunk_voxel_offset(1, 0, 0);
        let chunk_size = 32 * 32 * 32 * 4; // 4 bytes per voxel
        assert_eq!(offset2 - offset1, chunk_size as u64);
    }
    
    #[tokio::test]
    async fn test_unified_memory_manager() {
        let (device, _queue) = create_test_device().await;
        let device = std::sync::Arc::new(device);
        
        let manager = UnifiedMemoryManager::new(device.clone(), 8, 32); // Use test-safe values
        
        // Get memory stats
        let stats = manager.get_memory_stats();
        assert!(stats.total_allocated > 0);
        assert!(stats.voxel_data > 0);
        assert!(stats.chunk_metadata > 0);
        
        // Test bind group entry creation
        let entries = manager.create_bind_group_layout_entries(SystemType::TerrainGeneration);
        assert_eq!(entries.len(), 2); // voxel + metadata
        
        let entries = manager.create_bind_group_layout_entries(SystemType::Rendering);
        assert_eq!(entries.len(), 3); // voxel + metadata + lighting
    }
    
    #[tokio::test]
    async fn test_modification_command_creation() {
        // Test set block command
        let cmd = ModificationCommand::set_block(10, 20, 30, 42);
        assert_eq!(cmd.position, [10, 20, 30]);
        assert_eq!(cmd.block_id, 42);
        assert_eq!(cmd.mod_type, 0);
        
        // Test break block command
        let cmd = ModificationCommand::break_block(5, 15, 25);
        assert_eq!(cmd.position, [5, 15, 25]);
        assert_eq!(cmd.mod_type, 1);
        
        // Test explosion command
        let cmd = ModificationCommand::explode(100, 64, 100, 10.5);
        assert_eq!(cmd.position, [100, 64, 100]);
        assert_eq!(cmd.mod_type, 2);
        assert_eq!(cmd.radius, 10.5);
    }
    
    #[tokio::test]
    async fn test_terrain_params() {
        let default_params = TerrainParams::default();
        assert_eq!(default_params.seed, 12345);
        assert_eq!(default_params.sea_level, 64.0);
        assert_eq!(default_params.terrain_scale, 0.01);
        assert_eq!(default_params.mountain_threshold, 0.6);
        assert_eq!(default_params.cave_threshold, 0.3);
        assert_eq!(default_params.ore_chances, [0.1, 0.05, 0.02, 0.01]);
    }
    
    #[test]
    fn test_ao_value_conversion() {
        use crate::world_gpu::gpu_lighting::{extract_ao_from_metadata, ao_to_factor};
        
        // Test AO extraction
        let metadata = 0b10100111; // AO value in lower 4 bits = 7
        assert_eq!(extract_ao_from_metadata(metadata), 7);
        
        // Test AO to factor conversion
        assert_eq!(ao_to_factor(0), 1.0);      // No occlusion
        assert_eq!(ao_to_factor(15), 0.5);     // Full occlusion (50% darkening)
        assert!((ao_to_factor(7) - 0.7667).abs() < 0.001); // Partial occlusion
    }
}