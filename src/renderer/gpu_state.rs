use crate::{Camera, EngineConfig, Game, GameContext, BlockRegistry, BlockId, VoxelPos};
use crate::input::InputState;
use crate::physics::{PhysicsWorld, PlayerBody, MovementState, PhysicsBody};
use crate::renderer::{SelectionRenderer, SimpleAsyncRenderer, GpuDiagnostics, GpuInitProgress};
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
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
    view_proj: [[f32; 4]; 4],
    position: [f32; 3],
    _padding: f32,
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view: Matrix4::identity().into(),
            projection: Matrix4::identity().into(),
            view_proj: Matrix4::identity().into(),
            position: [0.0, 0.0, 0.0],
            _padding: 0.0,
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        let view = camera.build_view_matrix();
        let proj = camera.build_projection_matrix();
        self.view = view.into();
        self.projection = proj.into();
        self.view_proj = (proj * view).into();
        self.position = [camera.position.x, camera.position.y, camera.position.z];
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
    // Loading state
    first_chunks_loaded: bool,
    frames_rendered: u32,
    init_time: std::time::Instant,
}

impl GpuState {
    async fn new(window: Arc<Window>) -> Result<Self> {
        log::info!("[GpuState::new] Starting GPU initialization");
        let init_start = std::time::Instant::now();
        let progress = GpuInitProgress::new();
        
        let size = window.inner_size();
        log::debug!("[GpuState::new] Window size: {}x{}", size.width, size.height);

        // Create wgpu instance with timeout and diagnostics
        log::info!("[GpuState::new] Creating WGPU instance...");
        log::info!("[GpuState::new] Available backends: {:?}", wgpu::Backends::all());
        
        let instance_start = std::time::Instant::now();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let instance_time = instance_start.elapsed();
        log::info!("[GpuState::new] WGPU instance created in {:?}", instance_time);
        
        // Run comprehensive GPU diagnostics
        log::info!("[GpuState::new] Running GPU diagnostics...");
        let diagnostics_report = GpuDiagnostics::run_diagnostics(&instance).await;
        diagnostics_report.print_report();

        // Create surface with detailed error handling
        log::info!("[GpuState::new] Creating surface...");
        let surface_start = std::time::Instant::now();
        let surface = match instance.create_surface(window.clone()) {
            Ok(surf) => {
                let surface_time = surface_start.elapsed();
                log::info!("[GpuState::new] Surface created successfully in {:?}", surface_time);
                surf
            }
            Err(e) => {
                log::error!("[GpuState::new] Failed to create surface: {}", e);
                log::error!("[GpuState::new] This may be due to:");
                log::error!("[GpuState::new] - X11/Wayland display not available");
                log::error!("[GpuState::new] - WSL GPU passthrough not configured");
                log::error!("[GpuState::new] - Missing window system integration");
                return Err(anyhow::anyhow!("Surface creation failed: {}", e));
            }
        };

        // Request adapter with timeout and fallback options
        log::info!("[GpuState::new] Requesting GPU adapter...");
        let adapter_start = std::time::Instant::now();
        
        // Try high performance first
        let mut adapter_options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        };
        
        log::info!("[GpuState::new] Trying high-performance adapter...");
        let adapter_future = instance.request_adapter(&adapter_options);
        
        // WGPU has its own internal timeouts, so we don't need to add our own
        let adapter_result = adapter_future.await;
        
        let adapter = match adapter_result {
            Some(adapter) => {
                let adapter_time = adapter_start.elapsed();
                let info = adapter.get_info();
                log::info!("[GpuState::new] GPU adapter found in {:?}", adapter_time);
                log::info!("[GpuState::new] Adapter: {} ({:?})", info.name, info.device_type);
                log::info!("[GpuState::new] Backend: {:?}", info.backend);
                log::info!("[GpuState::new] Vendor: 0x{:04x}, Device: 0x{:04x}", info.vendor, info.device);
                adapter
            }
            None => {
                log::warn!("[GpuState::new] No high-performance adapter found, trying low power...");
                
                // Try low power adapter
                adapter_options.power_preference = wgpu::PowerPreference::LowPower;
                match instance.request_adapter(&adapter_options).await {
                    Some(adapter) => {
                        let info = adapter.get_info();
                        log::info!("[GpuState::new] Low-power adapter found: {}", info.name);
                        adapter
                    }
                    None => {
                        log::warn!("[GpuState::new] No low-power adapter found, trying fallback...");
                        
                        // Try fallback adapter
                        adapter_options.force_fallback_adapter = true;
                        match instance.request_adapter(&adapter_options).await {
                            Some(adapter) => {
                                let info = adapter.get_info();
                                log::warn!("[GpuState::new] Using fallback adapter: {}", info.name);
                                adapter
                            }
                            None => {
                                log::error!("[GpuState::new] No suitable GPU adapter found!");
                                log::error!("[GpuState::new] Tried: high-performance, low-power, and fallback adapters");
                                log::error!("[GpuState::new] This might be due to:");
                                log::error!("[GpuState::new] - No GPU available or GPU drivers not installed");
                                log::error!("[GpuState::new] - Running in WSL without GPU passthrough");
                                log::error!("[GpuState::new] - Incompatible graphics backend");
                                return Err(anyhow::anyhow!("No GPU adapter available"));
                            }
                        }
                    }
                }
            }
        };
        
        // Validate adapter capabilities
        log::info!("[GpuState::new] Validating adapter capabilities...");
        let validation_result = GpuDiagnostics::validate_capabilities(&adapter);
        validation_result.print_results();
        
        if !validation_result.is_valid {
            log::error!("[GpuState::new] GPU validation failed!");
            return Err(anyhow::anyhow!("GPU does not meet minimum requirements"));
        }

        // Create device and queue with timeout and validation
        log::info!("[GpuState::new] Requesting GPU device...");
        let device_start = std::time::Instant::now();
        
        // Query actual hardware limits first
        let adapter_limits = adapter.limits();
        let adapter_info = adapter.get_info();
        
        log::info!("[GpuState::new] Adapter hardware limits:");
        log::info!("[GpuState::new]   max_texture_dimension_2d: {}", adapter_limits.max_texture_dimension_2d);
        log::info!("[GpuState::new]   max_texture_dimension_3d: {}", adapter_limits.max_texture_dimension_3d);
        log::info!("[GpuState::new]   max_buffer_size: {} MB", adapter_limits.max_buffer_size / 1024 / 1024);
        log::info!("[GpuState::new]   max_vertex_buffers: {}", adapter_limits.max_vertex_buffers);
        log::info!("[GpuState::new]   max_bind_groups: {}", adapter_limits.max_bind_groups);
        log::info!("[GpuState::new]   max_compute_workgroup_size: {} x {} x {}", 
                  adapter_limits.max_compute_workgroup_size_x,
                  adapter_limits.max_compute_workgroup_size_y,
                  adapter_limits.max_compute_workgroup_size_z);
        
        // Detect GPU tier based on multiple factors
        let gpu_tier = determine_gpu_tier(&adapter_info, &adapter_limits);
        log::info!("[GpuState::new] Detected GPU tier: {:?}", gpu_tier);
        
        // Select appropriate limits based on GPU tier and actual capabilities
        let mut limits = select_limits_for_tier(gpu_tier, &adapter_limits);
        
        // For Earth Engine voxel rendering, optimize specific limits
        optimize_limits_for_voxel_engine(&mut limits, &adapter_limits, gpu_tier);
        
        log::info!("[GpuState::new] Final requested limits:");
        log::info!("[GpuState::new]   max_texture_2d: {} ({}x{})", 
                  limits.max_texture_dimension_2d,
                  limits.max_texture_dimension_2d,
                  limits.max_texture_dimension_2d);
        log::info!("[GpuState::new]   max_texture_3d: {}", limits.max_texture_dimension_3d);
        log::info!("[GpuState::new]   max_buffer_size: {} MB", limits.max_buffer_size / 1024 / 1024);
        log::info!("[GpuState::new]   max_vertex_buffers: {}", limits.max_vertex_buffers);
        log::info!("[GpuState::new]   max_bind_groups: {}", limits.max_bind_groups);
        log::info!("[GpuState::new]   max_vertex_attributes: {}", limits.max_vertex_attributes);
        log::info!("[GpuState::new]   max_uniform_buffer_binding_size: {} KB", 
                  limits.max_uniform_buffer_binding_size / 1024);
        
        let device_future = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                label: Some("Earth Engine Device"),
            },
            None,
        );
        
        // WGPU has its own internal timeouts, so we don't need to add our own
        let device_result = device_future.await;
        
        let (device, queue) = match device_result {
            Ok((dev, q)) => {
                let device_time = device_start.elapsed();
                log::info!("[GpuState::new] GPU device created successfully in {:?}", device_time);
                
                // Set up device error handler
                dev.on_uncaptured_error(Box::new(|error| {
                    log::error!("[GPU] Uncaptured device error: {:?}", error);
                    match error {
                        wgpu::Error::OutOfMemory { .. } => {
                            log::error!("[GPU] Out of GPU memory! Try reducing texture sizes or buffer allocations.");
                        }
                        wgpu::Error::Validation { description, .. } => {
                            log::error!("[GPU] Validation error: {}", description);
                        }
                        _ => {}
                    }
                }));
                
                // Run GPU operation tests
                log::info!("[GpuState::new] Testing GPU operations...");
                let test_results = GpuDiagnostics::test_gpu_operations(&dev).await;
                test_results.print_results();
                
                (dev, q)
            }
            Err(e) => {
                log::error!("[GpuState::new] Failed to create GPU device: {}", e);
                log::error!("[GpuState::new] This may be due to:");
                log::error!("[GpuState::new] - Requested features/limits not supported");
                log::error!("[GpuState::new] - GPU driver issues");
                log::error!("[GpuState::new] - Out of GPU memory");
                return Err(anyhow::anyhow!("Device creation failed: {}", e));
            }
        };

        // Configure surface with validation
        log::info!("[GpuState::new] Getting surface capabilities...");
        let surface_caps = surface.get_capabilities(&adapter);
        
        if surface_caps.formats.is_empty() {
            log::error!("[GpuState::new] No surface formats available!");
            return Err(anyhow::anyhow!("No surface formats supported"));
        }
        
        log::info!("[GpuState::new] Available surface formats: {:?}", surface_caps.formats);
        log::info!("[GpuState::new] Available present modes: {:?}", surface_caps.present_modes);
        log::info!("[GpuState::new] Available alpha modes: {:?}", surface_caps.alpha_modes);
        
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or_else(|| {
                log::warn!("[GpuState::new] No sRGB format found, using first available: {:?}", surface_caps.formats[0]);
                surface_caps.formats[0]
            });
        log::info!("[GpuState::new] Selected surface format: {:?}", surface_format);
        
        // Choose present mode with fallback
        let present_mode = if surface_caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo
        } else if surface_caps.present_modes.contains(&wgpu::PresentMode::AutoVsync) {
            log::warn!("[GpuState::new] Fifo not available, using AutoVsync");
            wgpu::PresentMode::AutoVsync
        } else {
            log::warn!("[GpuState::new] Using first available present mode: {:?}", surface_caps.present_modes[0]);
            surface_caps.present_modes[0]
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        log::info!("[GpuState::new] Configuring surface with size {}x{}...", config.width, config.height);
        let config_start = std::time::Instant::now();
        surface.configure(&device, &config);
        let config_time = config_start.elapsed();
        log::info!("[GpuState::new] Surface configured successfully in {:?}", config_time);

        // Create depth texture
        let depth_texture = create_depth_texture(&device, &config);

        // Create camera
        let camera = Camera::new(config.width, config.height);
        
        // Create camera uniform buffer
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);
        
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
        log::info!("[GpuState::new] Creating block registry...");
        let mut block_registry_mut = BlockRegistry::new();
        
        // Register basic blocks
        log::info!("[GpuState::new] Registering blocks...");
        let grass_id = block_registry_mut.register("test:grass", TestGrassBlock);
        let dirt_id = block_registry_mut.register("test:dirt", TestDirtBlock);
        let stone_id = block_registry_mut.register("test:stone", TestStoneBlock);
        let water_id = block_registry_mut.register("test:water", TestWaterBlock);
        let sand_id = block_registry_mut.register("test:sand", TestSandBlock);
        let _torch_id = block_registry_mut.register("test:torch", TestTorchBlock);
        log::info!("[GpuState::new] {} blocks registered", 6);
        
        let block_registry = Arc::new(block_registry_mut);
        
        // Create world with terrain generator
        log::info!("[GpuState::new] Creating world generator...");
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
        let cpu_count = num_cpus::get();
        log::info!("[GpuState::new] System has {} CPUs", cpu_count);
        
        let parallel_config = ParallelWorldConfig {
            generation_threads: cpu_count.saturating_sub(2).max(2),
            mesh_threads: cpu_count.saturating_sub(2).max(2),
            chunks_per_frame: cpu_count * 2,
            view_distance: 4,  // Balanced view distance for reasonable startup time
            chunk_size: 32,
        };
        
        log::info!("[GpuState::new] World config: {} gen threads, {} mesh threads, {} chunks/frame",
                  parallel_config.generation_threads, 
                  parallel_config.mesh_threads,
                  parallel_config.chunks_per_frame);
        
        // Store chunk_size before moving parallel_config
        let chunk_size = parallel_config.chunk_size;
        
        log::info!("[GpuState::new] Creating parallel world...");
        let mut world = ParallelWorld::new(generator, parallel_config);
        
        // Skip all pregeneration to ensure fast startup
        // Chunks will be generated asynchronously during the first frames
        log::info!("[GpuState::new] Skipping spawn area pregeneration for non-blocking startup");
        
        // Do one initial update to start chunk loading
        log::info!("[GpuState::new] Performing initial world update to queue chunk generation...");
        world.update(camera.position);
        log::info!("[GpuState::new] World initialization complete (chunk loading started)");
        
        // Create chunk renderer with async mesh building
        log::info!("[GpuState::new] Creating async chunk renderer...");
        let chunk_renderer = SimpleAsyncRenderer::new(
            Arc::clone(&block_registry),
            chunk_size,
            None, // Use default thread count
        );
        log::info!("[GpuState::new] Chunk renderer created");
        
        // Create selection renderer
        log::info!("[GpuState::new] Creating selection renderer...");
        let selection_renderer = SelectionRenderer::new(&device, config.format, &camera_bind_group_layout);
        log::info!("[GpuState::new] Selection renderer created");
        
        // Create physics world and player entity
        log::info!("[GpuState::new] Creating physics world...");
        let mut physics_world = PhysicsWorld::new();
        let player_body = PlayerBody::new(camera.position);
        let player_entity = physics_world.add_body(Box::new(player_body));
        log::info!("[GpuState::new] Physics world created with player entity");
        
        // Create lighting systems
        log::info!("[GpuState::new] Creating lighting systems...");
        let day_night_cycle = DayNightCycle::default(); // Starts at noon
        let light_propagator = LightPropagator::new();
        log::info!("[GpuState::new] Lighting systems created");

        let total_time = init_start.elapsed();
        log::info!("[GpuState::new] GPU state initialization complete in {:?}!", total_time);
        log::info!("[GpuState::new] GPU initialization summary:");
        log::info!("[GpuState::new] - Adapter: {}", adapter.get_info().name);
        log::info!("[GpuState::new] - Backend: {:?}", adapter.get_info().backend);
        log::info!("[GpuState::new] - Surface format: {:?}", surface_format);
        log::info!("[GpuState::new] - Present mode: {:?}", present_mode);
        
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
            first_chunks_loaded: false,
            frames_rendered: 0,
            init_time: std::time::Instant::now(),
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
            // Get device limits for texture size validation
            let device_limits = self.device.limits();
            let max_texture_dimension = device_limits.max_texture_dimension_2d;
            
            // Clamp window size to GPU texture limits
            let clamped_width = new_size.width.min(max_texture_dimension);
            let clamped_height = new_size.height.min(max_texture_dimension);
            
            // Log warnings if clamping occurred
            if new_size.width > max_texture_dimension {
                log::warn!(
                    "[GpuState::resize] Window width {} exceeds GPU texture limit {}, clamping to {}",
                    new_size.width,
                    max_texture_dimension,
                    clamped_width
                );
            }
            if new_size.height > max_texture_dimension {
                log::warn!(
                    "[GpuState::resize] Window height {} exceeds GPU texture limit {}, clamping to {}",
                    new_size.height,
                    max_texture_dimension,
                    clamped_height
                );
            }
            
            // Update size with clamped values
            self.size = winit::dpi::PhysicalSize::new(clamped_width, clamped_height);
            self.config.width = clamped_width;
            self.config.height = clamped_height;
            
            // Configure surface with validated size
            self.surface.configure(&self.device, &self.config);
            
            // Create depth texture with validated size
            self.depth_texture = create_depth_texture(&self.device, &self.config);
            
            // Update camera with validated size
            self.camera.resize(clamped_width, clamped_height);
        }
    }

    fn update_camera(&mut self) {
        self.camera_uniform.update_view_proj(&self.camera);
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

        // Track frames rendered
        self.frames_rendered += 1;
        
        // Log time to first frame
        if self.frames_rendered == 1 {
            let elapsed = self.init_time.elapsed();
            log::info!("[GpuState::render] First frame rendered in {:.2}ms", elapsed.as_millis());
        }

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
            
            // Check if we have chunks loaded
            let chunk_count = self.chunk_renderer.render(&mut render_pass, &self.camera);
            if chunk_count > 0 && !self.first_chunks_loaded {
                self.first_chunks_loaded = true;
                log::info!("[GpuState::render] First chunks rendered after {} frames", self.frames_rendered);
            } else if chunk_count == 0 && self.frames_rendered % 60 == 0 {
                // Log every second if no chunks are rendering
                log::warn!("[GpuState::render] No chunks rendered after {} frames (meshes: {}, queued: {})", 
                         self.frames_rendered, 
                         self.chunk_renderer.mesh_count(),
                         self.chunk_renderer.queued_builds());
            }
            
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

/// Validates and clamps texture dimensions to GPU limits
/// Following DOP principles - pure function that transforms data
/// Returns (clamped_width, clamped_height, was_clamped)
fn validate_texture_dimensions(
    requested_width: u32,
    requested_height: u32,
    max_dimension: u32,
) -> (u32, u32, bool) {
    let clamped_width = requested_width.min(max_dimension);
    let clamped_height = requested_height.min(max_dimension);
    let was_clamped = clamped_width != requested_width || clamped_height != requested_height;
    
    (clamped_width, clamped_height, was_clamped)
}

/// Creates a depth texture with validated dimensions
/// Pure function following DOP - transforms configuration data into texture view
fn create_depth_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> wgpu::TextureView {
    // Get device limits
    let device_limits = device.limits();
    let max_texture_dimension = device_limits.max_texture_dimension_2d;
    
    // Validate dimensions using pure function
    let (width, height, was_clamped) = validate_texture_dimensions(
        config.width,
        config.height,
        max_texture_dimension,
    );
    
    // Log if dimensions were clamped
    if was_clamped {
        log::warn!(
            "[create_depth_texture] Texture dimensions clamped from {}x{} to {}x{} due to GPU limits (max: {})",
            config.width, config.height, width, height, max_texture_dimension
        );
    }
    
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    
    log::debug!(
        "[create_depth_texture] Creating depth texture with size {}x{} (device limit: {})",
        width, height, max_texture_dimension
    );
    
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
    
    // Create texture with validated dimensions
    let texture = device.create_texture(&desc);
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

pub async fn run_app<G: Game + 'static>(
    event_loop: EventLoop<()>,
    config: EngineConfig,
    mut game: G,
) -> Result<()> {
    log::info!("[gpu_state::run_app] Starting GPU state initialization");
    
    // Don't reinit if already initialized
    if let Err(e) = env_logger::try_init() {
        log::debug!("[gpu_state::run_app] env_logger already initialized: {}", e);
    }

    log::info!("[gpu_state::run_app] Creating window...");
    let window = Arc::new(
        WindowBuilder::new()
            .with_title(&config.window_title)
            .with_inner_size(LogicalSize::new(config.window_width, config.window_height))
            .build(&event_loop)
            .map_err(|e| {
                log::error!("[gpu_state::run_app] Window creation failed: {}", e);
                e
            })?,
    );
    log::info!("[gpu_state::run_app] Window created successfully");

    log::info!("[gpu_state::run_app] Creating GPU state...");
    let mut gpu_state = match GpuState::new(window.clone()).await {
        Ok(state) => {
            log::info!("[gpu_state::run_app] GPU state created successfully");
            state
        }
        Err(e) => {
            log::error!("[gpu_state::run_app] GPU state creation failed: {}", e);
            return Err(e);
        }
    };
    
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

    // Request immediate redraw to render first frame ASAP
    log::info!("[gpu_state::run_app] Requesting initial redraw for immediate first frame");
    gpu_state.window.request_redraw();

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
                        // Always update world to ensure chunks are loaded and unloaded properly
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

/// GPU tier classification for intelligent limit selection
/// 
/// This system automatically detects GPU capabilities and selects appropriate
/// resource limits to maximize performance while maintaining compatibility.
/// 
/// # Tier Classifications:
/// 
/// - **HighEnd**: RTX 4070+, RX 7800+, M1/M2/M3 Pro/Max
///   - 8192x8192+ textures, 2GB+ buffers, all features enabled
///   - Special case: RTX 4060 Ti (despite name, has high-end capabilities)
/// 
/// - **MidRange**: RTX 4060/3070/3080, RX 7600/6600, standard M1/M2/M3
///   - 4096-8192 textures, 1GB buffers, most features enabled
/// 
/// - **Entry**: GTX 1660/2060, older RX cards, Intel Arc
///   - 4096 textures, 512MB buffers, core features only
/// 
/// - **LowEnd**: Intel Iris Xe, older integrated graphics
///   - 2048-4096 textures, 256MB buffers, minimal features
/// 
/// - **Fallback**: Software renderers, unknown GPUs
///   - WebGL2 compatible limits, maximum compatibility
#[derive(Debug, Clone, Copy, PartialEq)]
enum GpuTier {
    /// High-end modern GPUs (RTX 4070+, RX 7800+, etc)
    HighEnd,
    /// Mid-range modern GPUs (RTX 4060, RX 7600, etc)  
    MidRange,
    /// Entry-level or older GPUs
    Entry,
    /// Integrated graphics or very old GPUs
    LowEnd,
    /// Software renderer or unknown
    Fallback,
}

/// Determine GPU tier based on adapter info and capabilities
fn determine_gpu_tier(info: &wgpu::AdapterInfo, limits: &wgpu::Limits) -> GpuTier {
    // Check device type first
    if info.device_type == wgpu::DeviceType::Cpu {
        log::warn!("[GPU Tier] Software renderer detected");
        return GpuTier::Fallback;
    }
    
    // Vendor IDs
    const NVIDIA: u32 = 0x10DE;
    const AMD: u32 = 0x1002;
    const INTEL: u32 = 0x8086;
    const APPLE: u32 = 0x106B;
    
    // Check for modern GPU features and capabilities
    let has_high_texture_support = limits.max_texture_dimension_2d >= 16384;
    let has_large_buffers = limits.max_buffer_size >= 2 * 1024 * 1024 * 1024; // 2GB
    let has_many_bind_groups = limits.max_bind_groups >= 8;
    let name_lower = info.name.to_lowercase();
    
    log::info!("[GPU Tier] Analyzing GPU: {} (vendor: 0x{:04x})", info.name, info.vendor);
    
    match info.vendor {
        NVIDIA => {
            // NVIDIA GPU detection
            if name_lower.contains("rtx 40") || name_lower.contains("rtx 4080") || name_lower.contains("rtx 4090") {
                log::info!("[GPU Tier] Detected high-end NVIDIA RTX 40 series");
                GpuTier::HighEnd
            } else if name_lower.contains("rtx 4070") || name_lower.contains("rtx 4060") || 
                      name_lower.contains("rtx 30") || name_lower.contains("rtx 3070") || 
                      name_lower.contains("rtx 3080") || name_lower.contains("rtx 3090") {
                // RTX 4060 Ti has excellent capabilities despite mid-range positioning
                if name_lower.contains("rtx 4060 ti") && has_high_texture_support {
                    log::info!("[GPU Tier] Detected NVIDIA RTX 4060 Ti - using high-end profile");
                    GpuTier::HighEnd
                } else {
                    log::info!("[GPU Tier] Detected mid-range NVIDIA RTX");
                    GpuTier::MidRange
                }
            } else if name_lower.contains("rtx") || name_lower.contains("gtx 16") || 
                      name_lower.contains("gtx 20") {
                log::info!("[GPU Tier] Detected entry-level NVIDIA GPU");
                GpuTier::Entry
            } else if has_high_texture_support && has_large_buffers {
                // Unknown NVIDIA GPU but has good capabilities
                GpuTier::MidRange
            } else {
                GpuTier::LowEnd
            }
        }
        AMD => {
            // AMD GPU detection
            if name_lower.contains("rx 7900") || name_lower.contains("rx 7800") ||
               name_lower.contains("rx 6900") || name_lower.contains("rx 6800") {
                log::info!("[GPU Tier] Detected high-end AMD GPU");
                GpuTier::HighEnd
            } else if name_lower.contains("rx 7700") || name_lower.contains("rx 7600") ||
                      name_lower.contains("rx 6700") || name_lower.contains("rx 6600") ||
                      name_lower.contains("rx 5700") {
                log::info!("[GPU Tier] Detected mid-range AMD GPU");
                GpuTier::MidRange
            } else if name_lower.contains("rx") && has_high_texture_support {
                GpuTier::Entry
            } else {
                GpuTier::LowEnd
            }
        }
        INTEL => {
            // Intel GPU detection
            if name_lower.contains("arc a7") || name_lower.contains("arc a770") {
                log::info!("[GPU Tier] Detected high-end Intel Arc");
                GpuTier::MidRange
            } else if name_lower.contains("arc") {
                log::info!("[GPU Tier] Detected Intel Arc GPU");
                GpuTier::Entry
            } else if name_lower.contains("iris xe") || name_lower.contains("iris plus") {
                log::info!("[GPU Tier] Detected Intel Iris integrated graphics");
                GpuTier::LowEnd
            } else {
                log::info!("[GPU Tier] Detected Intel integrated graphics");
                GpuTier::Fallback
            }
        }
        APPLE => {
            // Apple Silicon detection
            if name_lower.contains("m2 pro") || name_lower.contains("m2 max") || 
               name_lower.contains("m3 pro") || name_lower.contains("m3 max") ||
               name_lower.contains("m1 pro") || name_lower.contains("m1 max") {
                log::info!("[GPU Tier] Detected high-end Apple Silicon");
                GpuTier::HighEnd
            } else if name_lower.contains("m1") || name_lower.contains("m2") || name_lower.contains("m3") {
                log::info!("[GPU Tier] Detected Apple Silicon");
                GpuTier::MidRange
            } else {
                GpuTier::Entry
            }
        }
        _ => {
            // Unknown vendor - use capabilities to determine tier
            log::info!("[GPU Tier] Unknown vendor, analyzing capabilities...");
            if has_high_texture_support && has_large_buffers && has_many_bind_groups {
                log::info!("[GPU Tier] Unknown GPU with high-end capabilities");
                GpuTier::MidRange
            } else if limits.max_texture_dimension_2d >= 8192 && limits.max_buffer_size >= 512 * 1024 * 1024 {
                log::info!("[GPU Tier] Unknown GPU with mid-range capabilities");
                GpuTier::Entry
            } else {
                log::info!("[GPU Tier] Unknown GPU with limited capabilities");
                GpuTier::LowEnd
            }
        }
    }
}

/// Select appropriate limits based on GPU tier
fn select_limits_for_tier(tier: GpuTier, hardware_limits: &wgpu::Limits) -> wgpu::Limits {
    match tier {
        GpuTier::HighEnd => {
            log::info!("[GPU Limits] Using high-end GPU profile");
            // Start with default limits (which are quite generous)
            let mut limits = wgpu::Limits::default();
            
            // But ensure we don't exceed hardware capabilities
            limits.max_texture_dimension_1d = limits.max_texture_dimension_1d.min(hardware_limits.max_texture_dimension_1d);
            limits.max_texture_dimension_2d = limits.max_texture_dimension_2d.min(hardware_limits.max_texture_dimension_2d);
            limits.max_texture_dimension_3d = limits.max_texture_dimension_3d.min(hardware_limits.max_texture_dimension_3d);
            limits.max_buffer_size = limits.max_buffer_size.min(hardware_limits.max_buffer_size);
            limits.max_vertex_buffers = limits.max_vertex_buffers.min(hardware_limits.max_vertex_buffers);
            limits.max_bind_groups = limits.max_bind_groups.min(hardware_limits.max_bind_groups);
            limits.max_vertex_attributes = limits.max_vertex_attributes.min(hardware_limits.max_vertex_attributes);
            limits.max_uniform_buffer_binding_size = limits.max_uniform_buffer_binding_size.min(hardware_limits.max_uniform_buffer_binding_size);
            
            limits
        }
        GpuTier::MidRange => {
            log::info!("[GPU Limits] Using mid-range GPU profile");
            // Use downlevel defaults as a base, but allow higher limits where available
            let mut limits = wgpu::Limits::downlevel_defaults();
            
            // Override specific limits for better performance on mid-range GPUs
            if hardware_limits.max_texture_dimension_2d >= 8192 {
                limits.max_texture_dimension_2d = 8192;
            }
            if hardware_limits.max_buffer_size >= 1024 * 1024 * 1024 {
                limits.max_buffer_size = 1024 * 1024 * 1024; // 1GB
            }
            
            // Ensure we don't exceed hardware
            limits.max_texture_dimension_1d = limits.max_texture_dimension_1d.min(hardware_limits.max_texture_dimension_1d);
            limits.max_texture_dimension_2d = limits.max_texture_dimension_2d.min(hardware_limits.max_texture_dimension_2d);
            limits.max_texture_dimension_3d = limits.max_texture_dimension_3d.min(hardware_limits.max_texture_dimension_3d);
            limits.max_buffer_size = limits.max_buffer_size.min(hardware_limits.max_buffer_size);
            limits.max_vertex_buffers = limits.max_vertex_buffers.min(hardware_limits.max_vertex_buffers);
            limits.max_bind_groups = limits.max_bind_groups.min(hardware_limits.max_bind_groups);
            
            limits
        }
        GpuTier::Entry => {
            log::info!("[GPU Limits] Using entry-level GPU profile");
            // Use downlevel defaults
            let mut limits = wgpu::Limits::downlevel_defaults();
            
            // Ensure minimum texture size for Earth Engine
            if hardware_limits.max_texture_dimension_2d >= 4096 {
                limits.max_texture_dimension_2d = 4096;
            }
            
            // Clamp to hardware
            limits.max_texture_dimension_1d = limits.max_texture_dimension_1d.min(hardware_limits.max_texture_dimension_1d);
            limits.max_texture_dimension_2d = limits.max_texture_dimension_2d.min(hardware_limits.max_texture_dimension_2d);
            limits.max_texture_dimension_3d = limits.max_texture_dimension_3d.min(hardware_limits.max_texture_dimension_3d);
            limits.max_buffer_size = limits.max_buffer_size.min(hardware_limits.max_buffer_size);
            
            limits
        }
        GpuTier::LowEnd | GpuTier::Fallback => {
            log::info!("[GPU Limits] Using low-end/fallback GPU profile");
            // Use WebGL2 defaults for maximum compatibility
            let mut limits = wgpu::Limits::downlevel_webgl2_defaults();
            
            // Still try to get 4096 textures if possible
            if hardware_limits.max_texture_dimension_2d >= 4096 {
                limits.max_texture_dimension_2d = 4096;
            }
            
            // Clamp to hardware
            limits.max_texture_dimension_1d = limits.max_texture_dimension_1d.min(hardware_limits.max_texture_dimension_1d);
            limits.max_texture_dimension_2d = limits.max_texture_dimension_2d.min(hardware_limits.max_texture_dimension_2d);
            limits.max_texture_dimension_3d = limits.max_texture_dimension_3d.min(hardware_limits.max_texture_dimension_3d);
            limits.max_buffer_size = limits.max_buffer_size.min(hardware_limits.max_buffer_size);
            
            limits
        }
    }
}

/// Optimize limits specifically for voxel engine requirements
fn optimize_limits_for_voxel_engine(limits: &mut wgpu::Limits, hardware_limits: &wgpu::Limits, tier: GpuTier) {
    log::info!("[GPU Limits] Optimizing for voxel engine...");
    
    // Texture requirements for voxel engines
    match tier {
        GpuTier::HighEnd | GpuTier::MidRange => {
            // Modern GPUs should use at least 8192x8192 for texture atlases
            if hardware_limits.max_texture_dimension_2d >= 8192 {
                limits.max_texture_dimension_2d = 8192;
                log::info!("[GPU Limits] Using 8192x8192 texture atlas support");
            } else if hardware_limits.max_texture_dimension_2d >= 4096 {
                limits.max_texture_dimension_2d = 4096;
                log::info!("[GPU Limits] Using 4096x4096 texture atlas support");
            }
            
            // 3D textures for volumetric effects
            if hardware_limits.max_texture_dimension_3d >= 512 {
                limits.max_texture_dimension_3d = 512;
            }
        }
        _ => {
            // Even low-end GPUs should try for 4096 if available
            if hardware_limits.max_texture_dimension_2d >= 4096 {
                limits.max_texture_dimension_2d = 4096;
                log::info!("[GPU Limits] Using 4096x4096 texture atlas support (minimum for quality)");
            } else {
                log::warn!("[GPU Limits] GPU doesn't support 4096x4096 textures, quality may be reduced");
            }
        }
    }
    
    // Buffer size requirements for chunk data
    match tier {
        GpuTier::HighEnd => {
            // High-end GPUs can handle large chunk buffers
            if hardware_limits.max_buffer_size >= 2 * 1024 * 1024 * 1024 {
                limits.max_buffer_size = 2 * 1024 * 1024 * 1024; // 2GB
                log::info!("[GPU Limits] Using 2GB buffer size for large world support");
            }
        }
        GpuTier::MidRange => {
            // Mid-range GPUs should have at least 1GB
            if hardware_limits.max_buffer_size >= 1024 * 1024 * 1024 {
                limits.max_buffer_size = 1024 * 1024 * 1024; // 1GB
                log::info!("[GPU Limits] Using 1GB buffer size");
            }
        }
        _ => {
            // Ensure minimum buffer size for chunk data
            let min_buffer_size = 256 * 1024 * 1024; // 256MB minimum
            if hardware_limits.max_buffer_size >= min_buffer_size {
                limits.max_buffer_size = limits.max_buffer_size.max(min_buffer_size);
                log::info!("[GPU Limits] Using {}MB buffer size", limits.max_buffer_size / 1024 / 1024);
            }
        }
    }
    
    // Uniform buffer requirements for rendering
    if tier == GpuTier::HighEnd || tier == GpuTier::MidRange {
        // Larger uniform buffers for more complex shaders
        if hardware_limits.max_uniform_buffer_binding_size >= 65536 {
            limits.max_uniform_buffer_binding_size = 65536;
            log::info!("[GPU Limits] Using 64KB uniform buffer size");
        }
    }
    
    // Compute workgroup sizes for GPU physics/lighting
    if hardware_limits.max_compute_workgroup_size_x >= 256 {
        log::info!("[GPU Limits] GPU supports 256+ compute workgroup size - good for parallel algorithms");
    }
    
    // Log any potential issues
    if limits.max_texture_dimension_2d < 4096 {
        log::warn!("[GPU Limits] Texture size below 4096x4096 may impact visual quality");
    }
    if limits.max_buffer_size < 256 * 1024 * 1024 {
        log::warn!("[GPU Limits] Buffer size below 256MB may limit world size");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_texture_dimensions() {
        // Test case 1: Dimensions within limits
        let (width, height, clamped) = validate_texture_dimensions(1024, 768, 4096);
        assert_eq!(width, 1024);
        assert_eq!(height, 768);
        assert!(!clamped);

        // Test case 2: Width exceeds limit
        let (width, height, clamped) = validate_texture_dimensions(8192, 768, 4096);
        assert_eq!(width, 4096);
        assert_eq!(height, 768);
        assert!(clamped);

        // Test case 3: Height exceeds limit
        let (width, height, clamped) = validate_texture_dimensions(1024, 8192, 4096);
        assert_eq!(width, 1024);
        assert_eq!(height, 4096);
        assert!(clamped);

        // Test case 4: Both dimensions exceed limit
        let (width, height, clamped) = validate_texture_dimensions(8192, 8192, 4096);
        assert_eq!(width, 4096);
        assert_eq!(height, 4096);
        assert!(clamped);

        // Test case 5: Exact limit dimensions
        let (width, height, clamped) = validate_texture_dimensions(4096, 4096, 4096);
        assert_eq!(width, 4096);
        assert_eq!(height, 4096);
        assert!(!clamped);

        // Test case 6: Very small GPU limit
        let (width, height, clamped) = validate_texture_dimensions(1920, 1080, 1024);
        assert_eq!(width, 1024);
        assert_eq!(height, 1024);
        assert!(clamped);
    }
}