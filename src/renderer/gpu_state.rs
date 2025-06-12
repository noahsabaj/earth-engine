use crate::{Camera, EngineConfig, Game, GameContext, BlockRegistry, BlockId, VoxelPos};
use crate::input::InputState;
use crate::physics::{PhysicsWorld, PlayerBody, MovementState, PhysicsBody};
use crate::renderer::{SelectionRenderer, SimpleAsyncRenderer};
use crate::world::{Ray, RaycastHit, ParallelWorld, ParallelWorldConfig};
use crate::lighting::{DayNightCycle, LightPropagator};
use anyhow::Result;
use cgmath::{Matrix4, SquareMatrix, Point3, Vector3, InnerSpace};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::{
    dpi::LogicalSize,
    event::{DeviceEvent, Event, WindowEvent, MouseButton},
    event_loop::EventLoop,
    keyboard::KeyCode,
    window::{CursorGrabMode, Window, WindowBuilder},
};

// Test blocks for initial rendering
struct TestGrassBlock;
impl crate::Block for TestGrassBlock {
    fn get_id(&self) -> crate::BlockId { crate::BlockId(1) }
    fn get_render_data(&self) -> crate::RenderData {
        crate::RenderData {
            color: [0.3, 0.7, 0.2],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> crate::PhysicsProperties {
        crate::PhysicsProperties {
            solid: true,
            density: 1200.0,
        }
    }
    fn get_name(&self) -> &str { "Grass" }
}

struct TestDirtBlock;
impl crate::Block for TestDirtBlock {
    fn get_id(&self) -> crate::BlockId { crate::BlockId(2) }
    fn get_render_data(&self) -> crate::RenderData {
        crate::RenderData {
            color: [0.5, 0.3, 0.1],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> crate::PhysicsProperties {
        crate::PhysicsProperties {
            solid: true,
            density: 1500.0,
        }
    }
    fn get_name(&self) -> &str { "Dirt" }
}

struct TestStoneBlock;
impl crate::Block for TestStoneBlock {
    fn get_id(&self) -> crate::BlockId { crate::BlockId(3) }
    fn get_render_data(&self) -> crate::RenderData {
        crate::RenderData {
            color: [0.6, 0.6, 0.6],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> crate::PhysicsProperties {
        crate::PhysicsProperties {
            solid: true,
            density: 2500.0,
        }
    }
    fn get_name(&self) -> &str { "Stone" }
}

struct TestWaterBlock;
impl crate::Block for TestWaterBlock {
    fn get_id(&self) -> crate::BlockId { crate::BlockId(6) }
    fn get_render_data(&self) -> crate::RenderData {
        crate::RenderData {
            color: [0.1, 0.4, 0.8],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> crate::PhysicsProperties {
        crate::PhysicsProperties {
            solid: false,
            density: 1000.0,
        }
    }
    fn get_name(&self) -> &str { "Water" }
}

struct TestSandBlock;
impl crate::Block for TestSandBlock {
    fn get_id(&self) -> crate::BlockId { crate::BlockId(5) }
    fn get_render_data(&self) -> crate::RenderData {
        crate::RenderData {
            color: [0.9, 0.8, 0.6],
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> crate::PhysicsProperties {
        crate::PhysicsProperties {
            solid: true,
            density: 1600.0,
        }
    }
    fn get_name(&self) -> &str { "Sand" }
}

struct TestTorchBlock;
impl crate::Block for TestTorchBlock {
    fn get_id(&self) -> crate::BlockId { crate::BlockId(7) }
    fn get_render_data(&self) -> crate::RenderData {
        crate::RenderData {
            color: [1.0, 0.8, 0.4], // Warm torch color
            texture_id: 0,
        }
    }
    fn get_physics_properties(&self) -> crate::PhysicsProperties {
        crate::PhysicsProperties {
            solid: false, // Can walk through torches
            density: 100.0,
        }
    }
    fn get_name(&self) -> &str { "Torch" }
    fn get_light_emission(&self) -> u8 { 14 } // Bright light
    fn is_transparent(&self) -> bool { true }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, view: Matrix4<f32>, proj: Matrix4<f32>) {
        self.view_proj = (proj * view).into();
    }
}

pub struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    depth_texture: wgpu::TextureView,
    // World and rendering
    world: ParallelWorld,
    block_registry: Arc<BlockRegistry>,
    chunk_renderer: SimpleAsyncRenderer,
    selection_renderer: SelectionRenderer,
    selected_block: Option<RaycastHit>,
    // Block breaking progress
    breaking_block: Option<VoxelPos>,
    breaking_progress: f32,
    // Physics
    physics_world: PhysicsWorld,
    player_entity: crate::physics::world::EntityId,
    // Lighting
    day_night_cycle: DayNightCycle,
    light_propagator: LightPropagator,
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface
        let surface = instance.create_surface(window.clone())?;

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find adapter"))?;

        // Create device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await?;

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Create depth texture
        let depth_texture = create_depth_texture(&device, &config);

        // Create camera
        let camera = Camera::new(config.width, config.height);
        
        // Create camera uniform buffer
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(
            camera.build_view_matrix(),
            camera.build_projection_matrix(),
        );
        
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        // Create render pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/voxel.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[crate::renderer::vertex::Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // Create world and registry
        let mut block_registry_mut = BlockRegistry::new();
        
        // Register basic blocks
        let grass_id = block_registry_mut.register("test:grass", TestGrassBlock);
        let dirt_id = block_registry_mut.register("test:dirt", TestDirtBlock);
        let stone_id = block_registry_mut.register("test:stone", TestStoneBlock);
        let water_id = block_registry_mut.register("test:water", TestWaterBlock);
        let sand_id = block_registry_mut.register("test:sand", TestSandBlock);
        let _torch_id = block_registry_mut.register("test:torch", TestTorchBlock);
        
        let block_registry = Arc::new(block_registry_mut);
        
        // Create world with terrain generator
        let seed = 12345; // Fixed seed for consistent worlds
        let generator = Box::new(crate::world::DefaultWorldGenerator::new(
            seed,
            grass_id,
            dirt_id,
            stone_id,
            water_id,
            sand_id,
        ));
        
        // Configure parallel world for better performance
        let parallel_config = ParallelWorldConfig {
            generation_threads: num_cpus::get().saturating_sub(2).max(2),
            mesh_threads: num_cpus::get().saturating_sub(2).max(2),
            chunks_per_frame: num_cpus::get() * 2,
            view_distance: 4,
            chunk_size: 32,
        };
        
        // Store chunk_size before moving parallel_config
        let chunk_size = parallel_config.chunk_size;
        
        let mut world = ParallelWorld::new(generator, parallel_config);
        
        // Pregenerate spawn area for smooth start
        world.pregenerate_spawn_area(camera.position, 2);
        
        // Initial update
        world.update(camera.position);
        
        // Create chunk renderer with async mesh building
        let chunk_renderer = SimpleAsyncRenderer::new(
            Arc::clone(&block_registry),
            chunk_size,
            None, // Use default thread count
        );
        
        // Create selection renderer
        let selection_renderer = SelectionRenderer::new(&device, config.format, &camera_bind_group_layout);
        
        // Create physics world and player entity
        let mut physics_world = PhysicsWorld::new();
        let player_body = PlayerBody::new(camera.position);
        let player_entity = physics_world.add_body(Box::new(player_body));
        
        // Create lighting systems
        let day_night_cycle = DayNightCycle::default(); // Starts at noon
        let light_propagator = LightPropagator::new();

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_bind_group_layout,
            depth_texture,
            world,
            block_registry,
            chunk_renderer,
            selection_renderer,
            selected_block: None,
            breaking_block: None,
            breaking_progress: 0.0,
            physics_world,
            player_entity,
            day_night_cycle,
            light_propagator,
        })
    }

    /// Cast ray through parallel world
    fn cast_ray_parallel(&self, ray: Ray, max_distance: f32) -> Option<RaycastHit> {
        // Use a simple ray casting implementation that works with ParallelWorld
        let chunk_manager = self.world.chunk_manager();
        let chunk_size = self.world.config().chunk_size;
        
        // Cast ray by checking blocks along the ray path
        let step = 0.1; // Step size for ray marching
        let steps = (max_distance / step) as i32;
        
        for i in 0..steps {
            let t = i as f32 * step;
            let pos = ray.origin + ray.direction * t;
            let voxel_pos = VoxelPos::new(
                pos.x.floor() as i32,
                pos.y.floor() as i32,
                pos.z.floor() as i32,
            );
            
            let block = self.world.get_block(voxel_pos);
            if block != BlockId::AIR {
                // Found a hit, determine which face
                let face = self.determine_hit_face(ray, voxel_pos, t);
                return Some(RaycastHit {
                    position: voxel_pos,
                    face,
                    distance: t,
                    block,
                });
            }
        }
        
        None
    }
    
    /// Determine which face of a block was hit by a ray
    fn determine_hit_face(&self, ray: Ray, block_pos: VoxelPos, distance: f32) -> crate::world::BlockFace {
        let hit_point = ray.origin + ray.direction * distance;
        let block_center = Point3::new(
            block_pos.x as f32 + 0.5,
            block_pos.y as f32 + 0.5,
            block_pos.z as f32 + 0.5,
        );
        
        let diff = hit_point - block_center;
        let abs_x = diff.x.abs();
        let abs_y = diff.y.abs();
        let abs_z = diff.z.abs();
        
        if abs_x > abs_y && abs_x > abs_z {
            if diff.x > 0.0 { crate::world::BlockFace::Right } else { crate::world::BlockFace::Left }
        } else if abs_y > abs_x && abs_y > abs_z {
            if diff.y > 0.0 { crate::world::BlockFace::Top } else { crate::world::BlockFace::Bottom }
        } else {
            if diff.z > 0.0 { crate::world::BlockFace::Front } else { crate::world::BlockFace::Back }
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture = create_depth_texture(&self.device, &self.config);
            self.camera.resize(new_size.width, new_size.height);
        }
    }

    fn update_camera(&mut self) {
        self.camera_uniform.update_view_proj(
            self.camera.build_view_matrix(),
            self.camera.build_projection_matrix(),
        );
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }
    
    fn update_chunk_renderer(&mut self, input: &InputState) {
        // Queue dirty chunks for async mesh building
        self.chunk_renderer.queue_dirty_chunks(&self.world, &self.camera);
        
        // Update the async renderer (process queue and upload meshes)
        self.chunk_renderer.update(&self.device);
        
        // Clean up GPU buffers for unloaded chunks
        self.chunk_renderer.cleanup_unloaded_chunks(&self.world);
        
        // World update handles chunk loading/unloading automatically
        self.world.update(self.camera.position);
    }
    
    fn process_input(&mut self, input: &InputState, delta_time: f32, active_block: BlockId) -> (Option<(VoxelPos, BlockId)>, Option<VoxelPos>) {
        // Get player body for movement
        if let Some(body) = self.physics_world.get_body_mut(self.player_entity) {
            if let Some(player_body) = body.as_any_mut().downcast_mut::<PlayerBody>() {
                // Calculate movement direction based on camera yaw
                let yaw_rad = cgmath::Rad::from(self.camera.yaw).0;
                let forward = Vector3::new(yaw_rad.cos(), 0.0, yaw_rad.sin());
                let right = Vector3::new(yaw_rad.sin(), 0.0, -yaw_rad.cos());
                
                let mut move_dir = Vector3::new(0.0, 0.0, 0.0);
                
                // Movement input
                if input.is_key_pressed(KeyCode::KeyW) {
                    move_dir += forward;
                }
                if input.is_key_pressed(KeyCode::KeyS) {
                    move_dir -= forward;
                }
                if input.is_key_pressed(KeyCode::KeyA) {
                    move_dir -= right;
                }
                if input.is_key_pressed(KeyCode::KeyD) {
                    move_dir += right;
                }
                
                // Normalize diagonal movement
                if move_dir.magnitude() > 0.0 {
                    move_dir = move_dir.normalize();
                }
                
                // Handle movement state changes
                if !player_body.is_in_water && !player_body.is_on_ladder {
                    if input.is_key_pressed(KeyCode::ControlLeft) && player_body.rigid_body.grounded {
                        // Sprint
                        if player_body.movement_state != MovementState::Sprinting {
                            player_body.set_movement_state(MovementState::Sprinting);
                        }
                    } else if input.is_key_pressed(KeyCode::ShiftLeft) {
                        // Crouch
                        if player_body.movement_state != MovementState::Crouching {
                            player_body.set_movement_state(MovementState::Crouching);
                        }
                    } else if player_body.movement_state != MovementState::Normal {
                        player_body.set_movement_state(MovementState::Normal);
                    }
                }
                
                // Apply movement to physics body
                player_body.move_horizontal(move_dir);
                
                // Handle vertical movement on ladders
                if player_body.is_on_ladder {
                    if input.is_key_pressed(KeyCode::KeyW) {
                        player_body.move_vertical_on_ladder(true);
                    } else if input.is_key_pressed(KeyCode::KeyS) {
                        player_body.move_vertical_on_ladder(false);
                    }
                }
                
                // Jump or swim up
                if input.is_key_pressed(KeyCode::Space) {
                    player_body.jump();
                }
                
                // Swim down
                if player_body.is_in_water && input.is_key_pressed(KeyCode::ShiftLeft) {
                    let mut vel = player_body.get_velocity();
                    vel.y = -player_body.swim_speed;
                    player_body.set_velocity(vel);
                }
            }
        }
        
        // Mouse look - only process if cursor is locked
        if input.is_cursor_locked() {
            let (dx, dy) = input.get_mouse_delta();
            let sensitivity = 0.5;
            self.camera.rotate(dx * sensitivity, -dy * sensitivity);
        }
        
        // Ray casting for block selection
        let ray = Ray::new(
            self.camera.position,
            self.camera.get_forward_vector(),
        );
        // Cast ray using parallel world's chunk manager
        self.selected_block = self.cast_ray_parallel(ray, 10.0);
        
        // Block interactions
        let mut broke_block = None;
        let mut placed_block = None;
        
        // Handle block breaking with progress
        if input.is_mouse_button_pressed(MouseButton::Left) {
            if let Some(hit) = &self.selected_block {
                // Check if we're still breaking the same block
                if self.breaking_block == Some(hit.position) {
                    // Get block hardness
                    let hardness = if let Some(block) = self.block_registry.get_block(hit.block) {
                        block.get_hardness()
                    } else {
                        1.0
                    };
                    
                    // Increase breaking progress
                    self.breaking_progress += delta_time / hardness;
                    
                    // Break block when progress reaches 1.0
                    if self.breaking_progress >= 1.0 {
                        // Store the broken block ID before removing it
                        let broken_block_id = self.world.get_block(hit.position);
                        self.world.set_block(hit.position, BlockId::AIR);
                        broke_block = Some((hit.position, broken_block_id));
                        self.breaking_block = None;
                        self.breaking_progress = 0.0;
                    }
                } else {
                    // Start breaking a new block
                    self.breaking_block = Some(hit.position);
                    self.breaking_progress = 0.0;
                }
            } else {
                // No block selected, reset breaking
                self.breaking_block = None;
                self.breaking_progress = 0.0;
            }
        } else {
            // Not holding left click, reset breaking
            self.breaking_block = None;
            self.breaking_progress = 0.0;
        }
        
        if input.is_mouse_button_pressed(MouseButton::Right) {
            // Place block (instant)
            if let Some(hit) = &self.selected_block {
                let place_pos = VoxelPos::new(
                    hit.position.x + hit.face.offset().x,
                    hit.position.y + hit.face.offset().y,
                    hit.position.z + hit.face.offset().z,
                );
                
                // Check if position is empty
                if self.world.get_block(place_pos) == BlockId::AIR {
                    self.world.set_block(place_pos, active_block);
                    placed_block = Some(place_pos);
                    // Reset breaking progress when placing
                    self.breaking_block = None;
                    self.breaking_progress = 0.0;
                }
            }
        }
        
        (broke_block, placed_block)
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.5,
                            g: 0.8,
                            b: 1.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Draw chunks with async rendering
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            self.chunk_renderer.render(&mut render_pass, &self.camera);
            
            // Draw selection highlight with breaking progress
            let breaking_progress = if self.breaking_block.is_some() {
                self.breaking_progress
            } else {
                0.0
            };
            self.selection_renderer.render(
                &mut render_pass,
                &self.camera_bind_group,
                self.selected_block.as_ref(),
                &self.queue,
                breaking_progress,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn create_depth_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };
    let desc = wgpu::TextureDescriptor {
        label: Some("Depth Texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    let texture = device.create_texture(&desc);
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

pub async fn run_app<G: Game + 'static>(
    event_loop: EventLoop<()>,
    config: EngineConfig,
    mut game: G,
) -> Result<()> {
    env_logger::init();

    let window = Arc::new(
        WindowBuilder::new()
            .with_title(&config.window_title)
            .with_inner_size(LogicalSize::new(config.window_width, config.window_height))
            .build(&event_loop)?,
    );

    let mut gpu_state = GpuState::new(window.clone()).await?;
    
    // Register game blocks
    // Note: Blocks are already registered in GpuState::new()
    // game.register_blocks(&mut gpu_state.block_registry);
    
    let mut input_state = InputState::new();
    let mut last_frame = std::time::Instant::now();
    
    // Start with cursor locked for FPS controls
    input_state.set_cursor_locked(true);
    match gpu_state.window.set_cursor_grab(CursorGrabMode::Locked) {
        Ok(_) => {
            gpu_state.window.set_cursor_visible(false);
        }
        Err(e) => {
            eprintln!("Initial cursor lock failed: {:?}. Trying confined mode...", e);
            gpu_state.window.set_cursor_grab(CursorGrabMode::Confined).ok();
            gpu_state.window.set_cursor_visible(false);
        }
    }

    event_loop
        .run(move |event, elwt| {
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == gpu_state.window.id() => match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(physical_size) => {
                        gpu_state.resize(*physical_size);
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if event.physical_key == winit::keyboard::PhysicalKey::Code(KeyCode::Escape) 
                            && event.state == winit::event::ElementState::Pressed {
                            // Toggle cursor lock with Escape
                            let locked = !input_state.is_cursor_locked();
                            input_state.set_cursor_locked(locked);
                            input_state.clear_mouse_delta(); // Clear any accumulated delta
                            
                            if locked {
                                // Use only Locked mode for proper FPS controls
                                match gpu_state.window.set_cursor_grab(CursorGrabMode::Locked) {
                                    Ok(_) => {
                                        gpu_state.window.set_cursor_visible(false);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to lock cursor: {:?}", e);
                                        // Fall back to confined if locked isn't supported
                                        gpu_state.window.set_cursor_grab(CursorGrabMode::Confined).ok();
                                        gpu_state.window.set_cursor_visible(false);
                                    }
                                }
                            } else {
                                gpu_state.window.set_cursor_grab(CursorGrabMode::None).ok();
                                gpu_state.window.set_cursor_visible(true);
                            }
                        }
                        
                        if let winit::keyboard::PhysicalKey::Code(keycode) = event.physical_key {
                            input_state.process_key(keycode, event.state);
                        }
                    }
                    WindowEvent::MouseInput { button, state, .. } => {
                        input_state.process_mouse_button(*button, *state);
                    }
                    WindowEvent::Focused(focused) => {
                        if !focused && input_state.is_cursor_locked() {
                            // Release cursor when window loses focus
                            input_state.set_cursor_locked(false);
                            input_state.clear_mouse_delta();
                            gpu_state.window.set_cursor_grab(CursorGrabMode::None).ok();
                            gpu_state.window.set_cursor_visible(true);
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        let now = std::time::Instant::now();
                        let delta_time = (now - last_frame).as_secs_f32();
                        last_frame = now;

                        // Update input and camera
                        let active_block = game.get_active_block();
                        let (broken_block_info, placed_block_pos) = gpu_state.process_input(&input_state, delta_time, active_block);
                        
                        // Update physics
                        gpu_state.physics_world.update(&gpu_state.world, delta_time);
                        
                        // Sync camera position with player physics body and check fall damage
                        if let Some(body) = gpu_state.physics_world.get_body_mut(gpu_state.player_entity) {
                            if let Some(player_body) = body.as_any_mut().downcast_mut::<PlayerBody>() {
                                let player_pos = player_body.get_position();
                                
                                // Check for fall damage when landing
                                if player_body.rigid_body.grounded && player_body.fall_start_y.is_some() {
                                    let damage = player_body.calculate_fall_damage();
                                    if damage > 0.0 {
                                        println!("Fall damage: {} HP", damage as i32);
                                        // In a real game, apply damage to player health here
                                    }
                                    player_body.fall_start_y = None;
                                }
                                
                                // Camera at eye level (0.72m offset from body center)
                                gpu_state.camera.position = Point3::new(
                                    player_pos.x,
                                    player_pos.y + 0.72,
                                    player_pos.z
                                );
                            }
                        }
                        
                        // Update loaded chunks based on player position
                        gpu_state.world.update(gpu_state.camera.position);
                        
                        // Update day/night cycle
                        gpu_state.day_night_cycle.update(delta_time);
                        
                        // Update block lighting if blocks were changed
                        if let Some((pos, block_id)) = broken_block_info {
                            // A block was broken - check if it was a light source
                            if let Some(block) = gpu_state.block_registry.get_block(block_id) {
                                if block.get_light_emission() > 0 {
                                    // Removed a light source
                                    gpu_state.light_propagator.remove_light(pos, crate::lighting::LightType::Block, block.get_light_emission());
                                }
                            }
                            // Update skylight column
                            crate::lighting::SkylightCalculator::update_column(&mut gpu_state.world, pos.x, pos.y, pos.z);
                        }
                        
                        if let Some(place_pos) = placed_block_pos {
                            if let Some(block) = gpu_state.block_registry.get_block(active_block) {
                                if block.get_light_emission() > 0 {
                                    // Placed a light source
                                    gpu_state.light_propagator.add_light(place_pos, crate::lighting::LightType::Block, block.get_light_emission());
                                }
                            }
                            // Update skylight column
                            crate::lighting::SkylightCalculator::update_column(&mut gpu_state.world, place_pos.x, place_pos.y, place_pos.z);
                        }
                        
                        // Process light propagation if needed
                        if broken_block_info.is_some() || placed_block_pos.is_some() {
                            gpu_state.light_propagator.propagate(&mut gpu_state.world);
                        }
                        
                        gpu_state.update_camera();
                        input_state.clear_mouse_delta();
                        
                        // Update async chunk renderer
                        gpu_state.update_chunk_renderer(&input_state);

                        // Update game with context
                        let mut ctx = GameContext {
                            world: &mut gpu_state.world,
                            registry: &gpu_state.block_registry,
                            camera: &gpu_state.camera,
                            input: &input_state,
                            selected_block: gpu_state.selected_block.clone(),
                        };
                        game.update(&mut ctx, delta_time);
                        
                        // Handle block callbacks
                        if let Some((pos, block_id)) = broken_block_info {
                            game.on_block_break(pos, block_id);
                        }
                        if let Some(place_pos) = placed_block_pos {
                            game.on_block_place(place_pos, active_block);
                        }

                        // Render
                        match gpu_state.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => gpu_state.resize(gpu_state.size),
                            Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                            Err(e) => eprintln!("Render error: {:?}", e),
                        }
                    }
                    _ => {}
                },
                Event::DeviceEvent {
                    event: DeviceEvent::MouseMotion { delta },
                    ..
                } => {
                    if input_state.is_cursor_locked() {
                        input_state.process_mouse_motion(delta);
                    }
                }
                Event::AboutToWait => {
                    gpu_state.window.request_redraw();
                }
                _ => {}
            }
        })?;

    Ok(())
}