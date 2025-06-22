//! GPU state management for the renderer
//!
//! This file contains complex camera usage that will be migrated to data-oriented design
//! in a future sprint. For now, we allow deprecated warnings to focus on other cleanup.

#![allow(deprecated)]

// Include constants from root constants.rs
include!("../../constants.rs");

use crate::camera::{
    build_projection_matrix, build_view_matrix, calculate_forward_vector, camera_rotate,
    init_camera, init_camera_with_spawn, update_aspect_ratio, CameraData,
};
use crate::game::{
    get_active_block_from_game, handle_block_break, handle_block_place, register_game_blocks,
    update_game, GameData,
};
use crate::gpu::{GpuErrorRecovery, SafeCommandEncoder};
use crate::input::InputState;
use crate::physics::GpuPhysicsWorld;
use crate::physics::{physics_tables::PhysicsFlags, EntityId};
use crate::renderer::mesh_utils::{
    create_simple_cube_indices, create_simple_cube_vertices, generate_chunk_terrain_mesh,
};
use crate::renderer::vertex::Vertex;
use crate::renderer::{
    gpu_driven::GpuDrivenRenderer,
    gpu_meshing::{create_gpu_meshing_state, GpuMeshingState},
    GpuDiagnostics, GpuInitProgress, SelectionRenderer,
};
use crate::world::compute::GpuLightPropagator;
use crate::world::lighting::{create_default_day_night_cycle, update_day_night_cycle};
use crate::world::lighting::{DayNightCycleData, LightType, LightUpdate};
use crate::world::{
    core::{Ray, RaycastHit},
    interfaces::{ChunkManager, WorldConfig, WorldInterface},
    management::{ParallelWorld, ParallelWorldConfig, SpawnFinder, UnifiedWorldManager},
};
use crate::{BlockId, BlockRegistry, EngineConfig, GameContext, VoxelPos};
use anyhow::Result;
use cgmath::{InnerSpace, Matrix4, Point3, SquareMatrix, Vector3, Zero};
use chrono;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::{
    dpi::LogicalSize,
    event::{DeviceEvent, Event, MouseButton, WindowEvent},
    event_loop::EventLoop,
    keyboard::KeyCode,
    window::{CursorGrabMode, Window, WindowBuilder},
};

// Engine's basic blocks are registered via register_basic_blocks()

// Dummy game for when no game is provided
// Default empty game data for engine-only mode
#[derive(Clone)]
struct DefaultGameData;

impl GameData for DefaultGameData {}

// Full camera data for CPU-side operations
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
    view_proj: [[f32; 4]; 4],
    position: [f32; 3],
    _padding: f32,
}

// Note: Both voxel.wgsl and gpu_driven.wgsl now use the full CameraUniform struct
// This ensures compatibility across all render pipelines

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

    // Deprecated Camera method removed - use update_view_proj_data instead

    fn update_view_proj_data(&mut self, camera: &CameraData) {
        let view = build_view_matrix(camera);
        let proj = build_projection_matrix(camera);
        self.view = view.into();
        self.projection = proj.into();
        self.view_proj = (proj * view).into();
        self.position = camera.position;

        // Log camera matrices for debugging
        log::debug!("[CameraUniform] Camera position: {:?}", camera.position);
        log::debug!("[CameraUniform] View matrix: {:?}", view);
        log::debug!("[CameraUniform] Projection matrix: {:?}", proj);
        log::debug!("[CameraUniform] View-Proj matrix: {:?}", self.view_proj);
    }
}

pub struct GpuState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    features: wgpu::Features,
    error_recovery: Arc<GpuErrorRecovery>,
    render_pipeline: wgpu::RenderPipeline,
    camera: CameraData,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    depth_texture: wgpu::TextureView,
    // World and rendering
    world: ParallelWorld,
    block_registry: Arc<BlockRegistry>,
    chunk_renderer: GpuDrivenRenderer,
    selection_renderer: SelectionRenderer,
    selected_block: Option<RaycastHit>,
    // Block breaking progress
    breaking_block: Option<VoxelPos>,
    breaking_progress: f32,
    // Physics (GPU-accelerated)
    physics_world: Option<GpuPhysicsWorld>,
    player_entity: EntityId,
    // Lighting
    day_night_cycle: DayNightCycleData,
    light_propagator: Option<GpuLightPropagator>,
    // GPU meshing
    gpu_meshing: Arc<GpuMeshingState>,
    // Loading state
    first_chunks_loaded: bool,
    frames_rendered: u32,
    init_time: std::time::Instant,
    // Dirty chunk tracking for incremental mesh updates
    dirty_chunks: std::collections::HashSet<crate::ChunkPos>,
    // Track which chunks have valid meshes
    chunks_with_meshes: std::collections::HashSet<crate::ChunkPos>,
    // Map chunk positions to GPU mesh buffer indices
    chunk_to_buffer_index: std::collections::HashMap<crate::ChunkPos, u32>,
    // Map chunk positions to index counts for CPU-generated meshes
    chunk_index_counts: std::collections::HashMap<crate::ChunkPos, u32>,
    // Render object submission tracking
    last_render_object_count: u32,
    frames_without_objects: u32,
    total_objects_submitted: u64,
    last_submission_time: std::time::Instant,
    // Frame rate limiting
    last_frame_time: Option<std::time::Instant>,
}

impl GpuState {
    /// Get the supported GPU features
    pub fn features(&self) -> wgpu::Features {
        self.features
    }

    async fn new(window: Arc<Window>) -> Result<Self> {
        let default_config = EngineConfig::default();
        Self::new_with_game(window, None::<&mut DefaultGameData>, default_config).await
    }

    async fn new_with_game<G: GameData>(
        window: Arc<Window>,
        game: Option<&mut G>,
        engine_config: EngineConfig,
    ) -> Result<Self> {
        log::info!("[GpuState::new] Starting GPU initialization");
        let init_start = std::time::Instant::now();
        let _progress = GpuInitProgress::new();

        let size = window.inner_size();
        log::debug!(
            "[GpuState::new] Window size: {}x{}",
            size.width,
            size.height
        );

        // Create wgpu instance with timeout and diagnostics
        log::info!("[GpuState::new] Creating WGPU instance...");
        log::info!(
            "[GpuState::new] Available backends: {:?}",
            wgpu::Backends::all()
        );

        let instance_start = std::time::Instant::now();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let instance_time = instance_start.elapsed();
        log::info!(
            "[GpuState::new] WGPU instance created in {:?}",
            instance_time
        );

        // Run comprehensive GPU diagnostics
        log::info!("[GpuState::new] Running GPU diagnostics...");
        let diagnostics_report = GpuDiagnostics::run_diagnostics(&instance).await;
        diagnostics_report.print_report();

        // Initialize GPU type registry
        log::info!("[GpuState::new] Initializing GPU type registry...");
        crate::gpu::automation::initialize_gpu_registry();

        // Create surface with detailed error handling
        log::info!("[GpuState::new] Creating surface...");
        let surface_start = std::time::Instant::now();
        let surface = match instance.create_surface(window.clone()) {
            Ok(surf) => {
                let surface_time = surface_start.elapsed();
                log::info!(
                    "[GpuState::new] Surface created successfully in {:?}",
                    surface_time
                );
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

        // CRITICAL FIX: Manually enumerate and select best hardware GPU adapter
        // The default request_adapter() was selecting software renderer over NVIDIA GPU
        log::info!("[GpuState::new] Enumerating all GPU adapters...");
        let adapter_start = std::time::Instant::now();

        let adapters = instance.enumerate_adapters(wgpu::Backends::all());
        log::info!("[GpuState::new] Found {} total adapters", adapters.len());

        // Log all available adapters for debugging
        for (i, adapter) in adapters.iter().enumerate() {
            let info = adapter.get_info();
            log::info!(
                "[GpuState::new] Adapter {}: {} ({:?}) - Backend: {:?}, Vendor: 0x{:04x}",
                i,
                info.name,
                info.device_type,
                info.backend,
                info.vendor
            );
        }

        // Smart adapter selection: Prioritize hardware GPUs over software renderers
        let mut best_adapter = None;
        let mut best_score = -1i32;

        for adapter in adapters {
            let info = adapter.get_info();

            // Check if adapter is compatible with our surface
            let surface_compatible = adapter.is_surface_supported(&surface);
            log::info!(
                "[GpuState::new] Adapter '{}' surface compatible: {}",
                info.name,
                surface_compatible
            );

            // Score adapters based on hardware vs software and vendor
            let mut score = 0i32;

            if !surface_compatible {
                log::warn!(
                    "[GpuState::new] Adapter '{}' not surface compatible",
                    info.name
                );

                // In WSL, NVIDIA GPUs often report as incompatible even when they might work
                // However, for safety, we should prefer compatible adapters
                if info.name.to_lowercase().contains("nvidia") || info.vendor == 0x10DE {
                    log::warn!("[GpuState::new] NVIDIA GPU detected but not surface compatible - likely WSL limitation");
                    score -= 500; // Reduce score for incompatible NVIDIA GPU
                } else {
                    continue; // Skip non-NVIDIA incompatible adapters entirely
                }
            }

            let name_lower = info.name.to_lowercase();

            // Hardware GPUs get massive bonus
            match info.device_type {
                wgpu::DeviceType::DiscreteGpu => score += 1000,
                wgpu::DeviceType::IntegratedGpu => score += 500,
                wgpu::DeviceType::VirtualGpu => score += 100,
                wgpu::DeviceType::Other => score += 50, // D3D12 wrapper shows as Other
                wgpu::DeviceType::Cpu => score -= 1000, // Software renderer penalty
            }

            // NVIDIA GPUs get priority (especially RTX series)
            if info.vendor == 0x10DE || name_lower.contains("nvidia") {
                score += 500;
                if name_lower.contains("rtx 40") {
                    score += 200; // RTX 40 series bonus
                } else if name_lower.contains("rtx") {
                    score += 100; // RTX series bonus
                }
            }

            // AMD discrete GPUs get good score
            if info.vendor == 0x1002 || name_lower.contains("amd") || name_lower.contains("radeon")
            {
                score += 300;
            }

            // Intel Arc gets medium score
            if (info.vendor == 0x8086 || name_lower.contains("intel")) && name_lower.contains("arc")
            {
                score += 200;
            }

            // Heavily penalize software renderers
            if name_lower.contains("llvmpipe")
                || name_lower.contains("software")
                || name_lower.contains("swiftshader")
                || info.device_type == wgpu::DeviceType::Cpu
            {
                score -= 2000;
            }

            // Backend preferences: Vulkan > DX12 > OpenGL > others
            match info.backend {
                wgpu::Backend::Vulkan => score += 20,
                wgpu::Backend::Dx12 => score += 15,
                wgpu::Backend::Gl => score += 10,
                wgpu::Backend::Metal => score += 10,
                _ => score += 0,
            }

            log::info!(
                "[GpuState::new] Adapter '{}' scored: {} points",
                info.name,
                score
            );

            if score > best_score {
                best_score = score;
                best_adapter = Some(adapter);
            }
        }

        let adapter = match best_adapter {
            Some(adapter) => {
                let adapter_time = adapter_start.elapsed();
                let info = adapter.get_info();
                log::info!(
                    "[GpuState::new] Selected best GPU adapter in {:?} (score: {})",
                    adapter_time,
                    best_score
                );
                log::info!(
                    "[GpuState::new] Adapter: {} ({:?})",
                    info.name,
                    info.device_type
                );
                log::info!("[GpuState::new] Backend: {:?}", info.backend);
                log::info!(
                    "[GpuState::new] Vendor: 0x{:04x}, Device: 0x{:04x}",
                    info.vendor,
                    info.device
                );

                // Special logging for NVIDIA GPU success
                if info.vendor == 0x10DE || info.name.to_lowercase().contains("nvidia") {
                    log::info!("[GpuState::new] ✅ SUCCESSFULLY SELECTED NVIDIA GPU!");
                    log::info!("[GpuState::new] ✅ Hardware acceleration enabled!");
                }

                adapter
            }
            None => {
                log::error!("[GpuState::new] No suitable GPU adapter found!");
                log::error!("[GpuState::new] This might be due to:");
                log::error!("[GpuState::new] - No GPU available or GPU drivers not installed");
                log::error!("[GpuState::new] - Running in WSL without GPU passthrough");
                log::error!("[GpuState::new] - All adapters incompatible with surface");
                return Err(anyhow::anyhow!("No GPU adapter available"));
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
        log::info!(
            "[GpuState::new]   max_texture_dimension_2d: {}",
            adapter_limits.max_texture_dimension_2d
        );
        log::info!(
            "[GpuState::new]   max_texture_dimension_3d: {}",
            adapter_limits.max_texture_dimension_3d
        );
        log::info!(
            "[GpuState::new]   max_buffer_size: {} MB",
            adapter_limits.max_buffer_size / 1024 / 1024
        );
        log::info!(
            "[GpuState::new]   max_vertex_buffers: {}",
            adapter_limits.max_vertex_buffers
        );
        log::info!(
            "[GpuState::new]   max_bind_groups: {}",
            adapter_limits.max_bind_groups
        );
        log::info!(
            "[GpuState::new]   max_compute_workgroup_size: {} x {} x {}",
            adapter_limits.max_compute_workgroup_size_x,
            adapter_limits.max_compute_workgroup_size_y,
            adapter_limits.max_compute_workgroup_size_z
        );

        // Detect GPU tier based on multiple factors
        let gpu_tier = determine_gpu_tier(&adapter_info, &adapter_limits);
        log::info!("[GpuState::new] Detected GPU tier: {:?}", gpu_tier);

        // Select appropriate limits based on GPU tier and actual capabilities
        let mut limits = select_limits_for_tier(gpu_tier, &adapter_limits);

        // For Hearth Engine voxel rendering, optimize specific limits
        optimize_limits_for_voxel_engine(&mut limits, &adapter_limits, gpu_tier);

        log::info!("[GpuState::new] Final requested limits:");
        log::info!(
            "[GpuState::new]   max_texture_2d: {} ({}x{})",
            limits.max_texture_dimension_2d,
            limits.max_texture_dimension_2d,
            limits.max_texture_dimension_2d
        );
        log::info!(
            "[GpuState::new]   max_texture_3d: {}",
            limits.max_texture_dimension_3d
        );
        log::info!(
            "[GpuState::new]   max_buffer_size: {} MB",
            limits.max_buffer_size / 1024 / 1024
        );
        log::info!(
            "[GpuState::new]   max_vertex_buffers: {}",
            limits.max_vertex_buffers
        );
        log::info!(
            "[GpuState::new]   max_bind_groups: {}",
            limits.max_bind_groups
        );
        log::info!(
            "[GpuState::new]   max_vertex_attributes: {}",
            limits.max_vertex_attributes
        );
        log::info!(
            "[GpuState::new]   max_uniform_buffer_binding_size: {} KB",
            limits.max_uniform_buffer_binding_size / 1024
        );

        // Check for required features
        let adapter_features = adapter.features();
        log::info!("[GpuState::new] Checking GPU features...");

        let mut required_features = wgpu::Features::empty();

        // Check if VERTEX_WRITABLE_STORAGE is supported
        if adapter_features.contains(wgpu::Features::VERTEX_WRITABLE_STORAGE) {
            log::info!("[GpuState::new]   ✓ VERTEX_WRITABLE_STORAGE supported");
            required_features |= wgpu::Features::VERTEX_WRITABLE_STORAGE;
        } else {
            log::warn!("[GpuState::new]   ✗ VERTEX_WRITABLE_STORAGE not supported - GPU culling may be limited");
        }

        let device_future = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features,
                required_limits: limits,
                label: Some("Hearth Engine Device"),
            },
            None,
        );

        // WGPU has its own internal timeouts, so we don't need to add our own
        let device_result = device_future.await;

        let (device, queue) = match device_result {
            Ok((dev, q)) => {
                let device_time = device_start.elapsed();
                log::info!(
                    "[GpuState::new] GPU device created successfully in {:?}",
                    device_time
                );

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
                    }
                }));

                // Run GPU operation tests
                log::info!("[GpuState::new] Testing GPU operations...");
                let test_results = GpuDiagnostics::test_gpu_operations(&dev).await;
                test_results.print_results();

                (Arc::new(dev), Arc::new(q))
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

        // Create GPU error recovery system
        let error_recovery = Arc::new(GpuErrorRecovery::new(device.clone(), queue.clone()));
        log::info!("[GpuState::new] GPU error recovery system initialized");

        // Configure surface with validation
        log::info!("[GpuState::new] Getting surface capabilities...");
        let mut surface_caps = surface.get_capabilities(&adapter);

        let surface_format = if surface_caps.formats.is_empty() {
            log::error!("[GpuState::new] No surface formats available!");
            log::warn!("[GpuState::new] Attempting fallback with default surface format...");

            // Fallback: Try to use a common format that should work on Windows
            let fallback_format = wgpu::TextureFormat::Bgra8UnormSrgb;
            surface_caps = wgpu::SurfaceCapabilities {
                formats: vec![fallback_format],
                present_modes: vec![wgpu::PresentMode::Fifo, wgpu::PresentMode::Immediate],
                alpha_modes: vec![wgpu::CompositeAlphaMode::Opaque],
                usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
            };

            log::info!(
                "[GpuState::new] Using fallback surface format: {:?}",
                fallback_format
            );
            fallback_format
        } else {
            log::info!(
                "[GpuState::new] Available surface formats: {:?}",
                surface_caps.formats
            );
            log::info!(
                "[GpuState::new] Available present modes: {:?}",
                surface_caps.present_modes
            );
            log::info!(
                "[GpuState::new] Available alpha modes: {:?}",
                surface_caps.alpha_modes
            );

            surface_caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or_else(|| {
                    log::warn!(
                        "[GpuState::new] No sRGB format found, using first available: {:?}",
                        surface_caps.formats[0]
                    );
                    surface_caps.formats[0]
                })
        };
        log::info!(
            "[GpuState::new] Selected surface format: {:?}",
            surface_format
        );

        // Choose present mode with fallback - prioritize Immediate for performance
        let present_mode = if surface_caps
            .present_modes
            .contains(&wgpu::PresentMode::Immediate)
        {
            log::info!("[GpuState::new] Using Immediate mode for maximum performance");
            wgpu::PresentMode::Immediate
        } else if surface_caps
            .present_modes
            .contains(&wgpu::PresentMode::Mailbox)
        {
            log::info!("[GpuState::new] Immediate not available, using Mailbox (good performance)");
            wgpu::PresentMode::Mailbox
        } else if surface_caps
            .present_modes
            .contains(&wgpu::PresentMode::Fifo)
        {
            log::warn!("[GpuState::new] Only Fifo available - may cause vsync blocking");
            wgpu::PresentMode::Fifo
        } else {
            log::warn!(
                "[GpuState::new] Using first available present mode: {:?}",
                surface_caps.present_modes[0]
            );
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

        log::info!(
            "[GpuState::new] Configuring surface with size {}x{}...",
            config.width,
            config.height
        );
        let config_start = std::time::Instant::now();

        // CRITICAL: Handle surface configuration failure gracefully
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            surface.configure(&device, &config);
        })) {
            Ok(_) => {
                let config_time = config_start.elapsed();
                log::info!(
                    "[GpuState::new] Surface configured successfully in {:?}",
                    config_time
                );
            }
            Err(e) => {
                log::error!("[GpuState::new] Surface configuration failed: {:?}", e);
                log::error!("[GpuState::new] This is likely due to WSL/OpenGL incompatibility with Windows GPU");
                log::error!("[GpuState::new] The game cannot run in this configuration");
                log::error!("[GpuState::new] SOLUTIONS:");
                log::error!("[GpuState::new] 1. Run on native Windows instead of WSL");
                log::error!("[GpuState::new] 2. Use Linux Mint as you suggested");
                log::error!("[GpuState::new] 3. Try with MESA_D3D12_DEFAULT_ADAPTER_NAME=nvidia environment variable");
                return Err(anyhow::anyhow!("Surface configuration failed - likely WSL/GPU incompatibility. Please run on native Windows or Linux instead of WSL."));
            }
        }

        // Create depth texture
        let depth_texture = create_depth_texture(&device, &config);

        // Create temporary camera for initial buffer creation
        // We'll update the position after we create the terrain generator
        let temp_camera = init_camera(config.width, config.height);

        // Create camera uniform buffer
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj_data(&temp_camera);

        // Create buffer with full camera uniform size (used by voxel.wgsl)
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        log::info!("[GpuState::new] Camera buffer created with size: {} bytes (full CameraUniform for voxel.wgsl)",
                   std::mem::size_of::<CameraUniform>());

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
        let shader_source = include_str!("../shaders/rendering/voxel.wgsl");
        let validated_shader =
            crate::gpu::automation::create_gpu_shader(&device, "voxel_shader", shader_source)?;

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
                module: &validated_shader.module,
                entry_point: "vs_main",
                buffers: &[crate::renderer::vertex::vertex_buffer_layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &validated_shader.module,
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

        // Register engine's basic blocks first
        log::info!("[GpuState::new] Registering engine basic blocks...");
        crate::world::core::register_basic_blocks(&mut block_registry_mut);

        // Register game blocks if game is provided
        if let Some(game) = game {
            log::info!("[GpuState::new] Game data provided - Registering game blocks...");
            // Register game blocks through the callback system
            crate::game::callbacks::execute_register_blocks(&mut block_registry_mut);
            log::info!("[GpuState::new] Game blocks registered successfully");
        } else {
            log::info!("[GpuState::new] No game data provided - Using default blocks only");
        }

        // Get block IDs (they are constants from BlockId)
        let grass_id = BlockId::GRASS;
        let dirt_id = BlockId::DIRT;
        let stone_id = BlockId::STONE;
        let water_id = BlockId::WATER;
        let sand_id = BlockId::SAND;

        let block_registry = Arc::new(block_registry_mut);

        // Create world generator (custom, factory, or default GPU generator)
        let generator = if let Some(custom_gen) = engine_config.world_generator {
            log::info!("[GpuState::new] Using custom world generator from EngineConfig");
            custom_gen
        } else if let Some(ref factory) = engine_config.world_generator_factory {
            log::info!("[GpuState::new] Using world generator factory from EngineConfig");
            log::info!(
                "[GpuState::new] World generator type: {:?}",
                engine_config.world_generator_type
            );
            let generated = (factory)(device.clone(), queue.clone(), &engine_config);
            log::info!("[GpuState::new] World generator factory returned successfully");
            generated
        } else {
            log::info!("[GpuState::new] Creating default GPU-powered world generator...");
            let seed = 12345u32; // Fixed seed for consistent worlds
            Box::new(crate::world::generation::DefaultWorldGenerator::new(seed))
        };

        // Start with a temporary camera position (will be updated after spawn search)
        let temp_spawn_x = 0.0;
        let temp_spawn_z = 0.0;
        let temp_spawn_y = 80.0; // Temporary height above typical terrain

        // Create camera at temporary position
        let mut camera = init_camera_with_spawn(
            engine_config.window_width,
            engine_config.window_height,
            temp_spawn_x,
            temp_spawn_y,
            temp_spawn_z,
        );
        log::info!(
            "[GpuState::new] Camera created at temporary position: {:?}",
            camera.position
        );

        // Update camera uniform with actual camera position
        camera_uniform.update_view_proj_data(&camera);
        queue.write_buffer(&camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));

        // Configure parallel world for better performance
        let cpu_count = num_cpus::get();
        log::info!("[GpuState::new] System has {} CPUs", cpu_count);

        let parallel_config = ParallelWorldConfig {
            generation_threads: cpu_count.saturating_sub(2).max(2),
            mesh_threads: cpu_count.saturating_sub(2).max(2),
            chunks_per_frame: cpu_count * 2,
            view_distance: engine_config.render_distance as i32,
            chunk_size: engine_config.chunk_size,
            enable_gpu: true,
        };

        log::info!(
            "[GpuState::new] World config: {} gen threads, {} mesh threads, {} chunks/frame",
            parallel_config.generation_threads,
            parallel_config.mesh_threads,
            parallel_config.chunks_per_frame
        );

        // Store chunk_size before moving parallel_config
        let _chunk_size = parallel_config.chunk_size;

        log::info!("[GpuState::new] Creating parallel world...");
        let world_future = ParallelWorld::new(
            parallel_config,
            generator,
            Some(device.clone()),
            Some(queue.clone()),
        );
        let mut world = pollster::block_on(world_future)
            .map_err(|e| anyhow::anyhow!("Failed to create parallel world: {}", e))?;

        // Find safe spawn position by checking actual blocks
        log::info!("[GpuState::new] Finding safe spawn position...");
        let spawn_result = SpawnFinder::find_safe_spawn(&world, temp_spawn_x, temp_spawn_z, 10);

        let safe_spawn_pos = match spawn_result {
            Some(pos) => {
                log::info!("[GpuState::new] Found safe spawn position at {:?}", pos);
                SpawnFinder::debug_blocks_at_position(&world, pos);
                pos
            }
            None => {
                log::error!("[GpuState::new] Failed to find safe spawn position");
                log::warn!("[GpuState::new] Using fallback spawn position");
                Point3::new(temp_spawn_x, temp_spawn_y, temp_spawn_z)
            }
        };

        // Update camera to safe spawn position
        camera.position = [safe_spawn_pos.x, safe_spawn_pos.y, safe_spawn_pos.z];
        log::info!(
            "[GpuState::new] Camera moved to safe spawn position: {:?}",
            camera.position
        );

        // Update camera uniform with new position
        camera_uniform.update_view_proj_data(&camera);
        queue.write_buffer(&camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));

        // Do one initial update to start chunk loading
        log::info!("[GpuState::new] Performing initial world update to queue chunk generation...");
        log::info!(
            "[GpuState::new] Camera position for initial update: {:?}",
            camera.position
        );

        // Ensure camera chunk is loaded before initial world update
        let initial_camera_pos =
            Point3::new(camera.position[0], camera.position[1], camera.position[2]);
        let camera_chunk_loaded = world.ensure_camera_chunk_loaded(initial_camera_pos);
        if camera_chunk_loaded {
            log::info!("[GpuState::new] Camera chunk successfully loaded at initialization");
        } else {
            log::warn!("[GpuState::new] Camera chunk still being generated at initialization");
        }

        world.update(initial_camera_pos);
        log::info!("[GpuState::new] World initialization complete (chunk loading started)");

        // Create GPU-driven renderer
        log::info!("[GpuState::new] Creating GPU-driven renderer...");
        let chunk_renderer = GpuDrivenRenderer::new(
            device.clone(),
            queue.clone(),
            config.format,
            &camera_bind_group_layout,
        );
        log::info!("[GpuState::new] GPU-driven renderer created");

        // Create selection renderer
        log::info!("[GpuState::new] Creating selection renderer...");
        let selection_renderer =
            SelectionRenderer::new(&device, config.format, &camera_bind_group_layout);
        log::info!("[GpuState::new] Selection renderer created");

        // Create GPU meshing system
        log::info!("[GpuState::new] Creating GPU meshing system...");
        let gpu_meshing = Arc::new(create_gpu_meshing_state(device.clone(), queue.clone()));
        log::info!("[GpuState::new] GPU meshing system created");

        // Set GPU meshing reference on the chunk renderer
        let mut chunk_renderer = chunk_renderer;
        chunk_renderer.set_gpu_meshing(gpu_meshing.clone());

        // Create physics world and player entity
        log::info!("[GpuState::new] Creating GPU physics world...");

        // Create GPU physics system if WorldBuffer is available
        let (physics_world, player_entity) = if let Some(world_buffer) = world.get_world_buffer() {
            log::info!("[GpuState::new] Creating GPU-accelerated physics system...");
            let mut gpu_physics = GpuPhysicsWorld::new(
                device.clone(),
                queue.clone(),
                1024, // Max entities
            )?;

            // Integrate with GPU WorldBuffer
            gpu_physics.set_world_buffer(world_buffer);

            let player_entity = gpu_physics.add_entity(
                Point3::new(camera.position[0], camera.position[1], camera.position[2]),
                Vector3::zero(),
                Vector3::new(0.8, 1.8, 0.8), // Player size
                80.0,                        // Mass in kg
                0.8,                         // Friction
                0.0,                         // Restitution
            );

            (Some(gpu_physics), player_entity)
        } else {
            log::warn!("[GpuState::new] WorldBuffer not available - falling back to CPU physics");
            // Fallback player entity ID
            (None, EntityId(1))
        };

        log::info!("[GpuState::new] Physics world created with player entity ID: {} at safe spawn position: {:?}", player_entity, camera.position);

        // Print movement instructions for user
        log::info!("=== MOVEMENT CONTROLS ===");
        log::info!("WASD - Move around");
        log::info!("Mouse - Look around (click in window to lock cursor)");
        log::info!("Space - Jump");
        log::info!("Shift - Sprint");
        log::info!("Ctrl - Crouch");
        log::info!("Escape - Toggle cursor lock");
        log::info!("========================");

        // Verify the entity was added correctly
        if let Some(ref physics_world) = physics_world {
            if let Some(body) = physics_world.get_body(player_entity) {
                log::info!(
                    "[GpuState::new] Player body verified at position: [{:.2}, {:.2}, {:.2}]",
                    body.position[0],
                    body.position[1],
                    body.position[2]
                );
                let is_grounded = (body.flags & PhysicsFlags::GROUNDED) != 0;
                if !is_grounded {
                    log::info!("[GpuState::new] Player spawned in air - will fall to ground and become grounded");
                }
            } else {
                log::error!("[GpuState::new] Failed to retrieve player body after creation!");
            }
        } else {
            log::warn!("[GpuState::new] GPU physics world not available - using fallback physics");
        }

        // Create lighting systems
        log::info!("[GpuState::new] Creating lighting systems...");
        let day_night_cycle = create_default_day_night_cycle(); // Starts at noon

        // Create GPU light propagator if WorldBuffer is available
        let light_propagator = if let Some(_world_buffer) = world.get_world_buffer() {
            log::info!("[GpuState::new] GPU lighting system available");
            // TODO: Update GpuLightPropagator to work with world::storage::WorldBuffer
            log::warn!("[GpuState::new] GpuLightPropagator not yet ported to world_unified - using CPU fallback");
            None
        } else {
            log::warn!("[GpuState::new] WorldBuffer not available - falling back to CPU lighting");
            None
        };
        log::info!("[GpuState::new] Lighting systems created");

        let total_time = init_start.elapsed();
        log::info!(
            "[GpuState::new] GPU state initialization complete in {:?}!",
            total_time
        );
        log::info!("[GpuState::new] GPU initialization summary:");
        log::info!("[GpuState::new] - Adapter: {}", adapter.get_info().name);
        log::info!(
            "[GpuState::new] - Backend: {:?}",
            adapter.get_info().backend
        );
        log::info!("[GpuState::new] - Surface format: {:?}", surface_format);
        log::info!("[GpuState::new] - Present mode: {:?}", present_mode);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            error_recovery,
            size,
            features: required_features,
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
            gpu_meshing,
            first_chunks_loaded: false,
            frames_rendered: 0,
            init_time: std::time::Instant::now(),
            dirty_chunks: std::collections::HashSet::new(),
            chunks_with_meshes: std::collections::HashSet::new(),
            chunk_to_buffer_index: std::collections::HashMap::new(),
            chunk_index_counts: std::collections::HashMap::new(),
            last_render_object_count: 0,
            frames_without_objects: 0,
            total_objects_submitted: 0,
            last_submission_time: std::time::Instant::now(),
            last_frame_time: None,
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
    fn determine_hit_face(
        &self,
        ray: Ray,
        block_pos: VoxelPos,
        distance: f32,
    ) -> crate::world::BlockFace {
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
            if diff.x > 0.0 {
                crate::world::BlockFace::Right
            } else {
                crate::world::BlockFace::Left
            }
        } else if abs_y > abs_x && abs_y > abs_z {
            if diff.y > 0.0 {
                crate::world::BlockFace::Top
            } else {
                crate::world::BlockFace::Bottom
            }
        } else {
            if diff.z > 0.0 {
                crate::world::BlockFace::Front
            } else {
                crate::world::BlockFace::Back
            }
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
            self.camera = update_aspect_ratio(&self.camera, clamped_width, clamped_height);
        }
    }

    fn update_camera(&mut self) {
        self.camera_uniform.update_view_proj_data(&self.camera);

        // Write the full camera uniform to the buffer
        // This is required by voxel.wgsl which expects all camera data
        log::trace!(
            "[GpuState::update_camera] Writing full camera uniform: {} bytes",
            std::mem::size_of::<CameraUniform>()
        );

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    fn update_chunk_renderer(&mut self, _input: &InputState) {
        // Begin new frame for GPU-driven renderer
        self.chunk_renderer.begin_frame(&self.camera);

        // Get chunk size from config
        let chunk_size = self.world.config().chunk_size;

        // Recovery mechanism: Force dirty all chunks if no objects for too long
        if self.frames_without_objects >= 600 && self.chunks_with_meshes.len() > 0 {
            log::warn!(
                "[GpuState::update_chunk_renderer] Attempting recovery - marking all {} chunks as dirty",
                self.chunks_with_meshes.len()
            );

            // Mark all chunks with meshes as dirty to force rebuild
            for chunk_pos in self.chunks_with_meshes.clone() {
                self.dirty_chunks.insert(chunk_pos);
            }

            // Clear chunks_with_meshes and mapping to force complete rebuild
            self.chunks_with_meshes.clear();
            self.chunk_to_buffer_index.clear();
        }

        // Process dirty chunks and new chunks
        let mut render_objects = Vec::new();

        // Check all loaded chunks
        let mut chunks_needing_rebuild = Vec::new();
        let loaded_chunks: Vec<_> = self.world.iter_loaded_chunks().collect();
        let loaded_count = loaded_chunks.len();

        // Log diagnostic info every 60 frames
        if self.frames_rendered % 60 == 0 {
            log::info!(
                "[GpuState::update_chunk_renderer] Frame {}: Loaded chunks: {}, Chunks with meshes: {}, Dirty chunks: {}",
                self.frames_rendered, loaded_count, self.chunks_with_meshes.len(), self.dirty_chunks.len()
            );

            // Log first few chunk positions for debugging
            if loaded_count > 0 {
                let first_chunks: Vec<_> =
                    loaded_chunks.iter().take(3).map(|(pos, _)| *pos).collect();
                log::info!(
                    "[GpuState::update_chunk_renderer] First loaded chunks: {:?}",
                    first_chunks
                );
            }
        }

        for (chunk_pos, _chunk_lock) in loaded_chunks {
            // Check if this chunk needs rebuilding
            let needs_rebuild = self.dirty_chunks.contains(&chunk_pos)
                || !self.chunks_with_meshes.contains(&chunk_pos);

            if needs_rebuild {
                chunks_needing_rebuild.push(chunk_pos);
            }
        }

        if chunks_needing_rebuild.len() > 0 {
            log::trace!(
                "[GpuState::update_chunk_renderer] {} chunks need rebuilding out of {} loaded chunks",
                chunks_needing_rebuild.len(),
                loaded_count
            );
        }

        // Process chunks that need rebuilding
        let chunks_to_rebuild_count = chunks_needing_rebuild.len();
        if chunks_to_rebuild_count > 0 && self.frames_rendered % 60 == 0 {
            log::info!(
                "[GpuState::update_chunk_renderer] Processing {} chunks that need rebuilding",
                chunks_to_rebuild_count
            );
        }

        // GPU-only mesh generation
        if chunks_needing_rebuild.len() > 0 {
            log::info!(
                "[GpuState::update_chunk_renderer] Generating {} chunk meshes on GPU",
                chunks_needing_rebuild.len()
            );

            // Get world buffer for GPU mesh generation
            log::info!("[GpuState::update_chunk_renderer] Attempting to get world buffer...");
            let world_buffer_opt = self.world.get_world_buffer();
            log::info!(
                "[GpuState::update_chunk_renderer] World buffer result: {}",
                if world_buffer_opt.is_some() {
                    "Some(buffer)"
                } else {
                    "None"
                }
            );

            // TEMPORARY: Force CPU fallback until GPU pipeline issues are resolved
            let force_cpu_fallback = true;

            // Check if GPU world buffer is available and not forcing CPU fallback
            if !force_cpu_fallback {
                if let Some(world_buffer) = world_buffer_opt {
                    log::info!(
                        "[GpuState::update_chunk_renderer] Got world buffer, attempting to lock..."
                    );
                    match world_buffer.lock() {
                        Ok(wb) => {
                            log::info!("[GpuState::update_chunk_renderer] World buffer locked, calling generate_chunk_meshes...");
                            // Generate meshes on GPU
                            let mesh_results = crate::renderer::gpu_meshing::generate_chunk_meshes(
                                &self.gpu_meshing,
                                wb.voxel_buffer(),
                                &chunks_needing_rebuild,
                                0, // LOD level 0 (full detail)
                            );

                            // Process mesh generation results
                            log::info!("[GpuState::update_chunk_renderer] Processing {} mesh generation results",
                              mesh_results.len());

                            for result in mesh_results {
                                let chunk_pos = result.chunk_pos;

                                log::trace!("[GpuState::update_chunk_renderer] Processing mesh result for chunk {:?}, buffer_index: {}",
                                  chunk_pos, result.buffer_index);

                                // Get the mesh buffer for this chunk
                                if let Some(mesh_buffer) =
                                    crate::renderer::gpu_meshing::get_mesh_buffer(
                                        &self.gpu_meshing,
                                        result.buffer_index,
                                    )
                                {
                                    // Create render object for GPU-driven renderer
                                    let render_object = crate::renderer::gpu_driven::RenderObject {
                                        position: cgmath::Vector3::new(
                                            (chunk_pos.x * chunk_size as i32
                                                + chunk_size as i32 / 2)
                                                as f32,
                                            (chunk_pos.y * chunk_size as i32
                                                + chunk_size as i32 / 2)
                                                as f32,
                                            (chunk_pos.z * chunk_size as i32
                                                + chunk_size as i32 / 2)
                                                as f32,
                                        ),
                                        scale: 1.0,
                                        color: [1.0, 1.0, 1.0, 1.0],
                                        bounding_radius: (chunk_size as f32 * 1.732) / 2.0, // Radius of chunk bounding sphere
                                        mesh_id: result.buffer_index,
                                        material_id: 0,
                                        index_count: None, // GPU-generated meshes determine their own index count
                                    };

                                    // GPU mesh buffers are managed by the GPU meshing system
                                    // The renderer will query them directly when needed

                                    render_objects.push(render_object);

                                    // Mark chunk as processed and store buffer index mapping
                                    self.dirty_chunks.remove(&chunk_pos);
                                    self.chunks_with_meshes.insert(chunk_pos);
                                    self.chunk_to_buffer_index
                                        .insert(chunk_pos, result.buffer_index);

                                    log::trace!("[GpuState::update_chunk_renderer] GPU mesh generation completed for chunk {:?}, buffer_index: {}", chunk_pos, result.buffer_index);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("[GpuState::update_chunk_renderer] Failed to lock world buffer for GPU mesh generation: {:?}", e);
                        }
                    }
                }
            } else {
                log::warn!("[GpuState::update_chunk_renderer] 🔄 CPU FALLBACK TRIGGERED - Force flag: {}, World buffer: {}",
                         force_cpu_fallback,
                         if world_buffer_opt.is_some() { "Available" } else { "None" });

                // Fallback: Create simple CPU meshes for visualization
                log::warn!("[GpuState::update_chunk_renderer] 🏗️ Using CPU mesh generation fallback for {} chunks",
                         chunks_needing_rebuild.len());

                // Allocate mesh buffers and generate simple cube meshes
                let mut allocator = self.gpu_meshing.allocator.lock().unwrap();

                for chunk_pos in chunks_needing_rebuild {
                    // Ensure chunk is loaded before generating mesh
                    let is_loaded = self.world.is_chunk_loaded(chunk_pos);
                    log::debug!(
                        "[GpuState::update_chunk_renderer] Chunk {:?} loaded status: {}",
                        chunk_pos,
                        is_loaded
                    );

                    if !is_loaded {
                        log::info!("[GpuState::update_chunk_renderer] 📁 Loading chunk {:?} before mesh generation", chunk_pos);
                        if let Err(e) = self.world.load_chunk(chunk_pos) {
                            log::error!(
                                "[GpuState::update_chunk_renderer] Failed to load chunk {:?}: {:?}",
                                chunk_pos,
                                e
                            );
                            continue;
                        }
                    } else {
                        // Force load even if marked as loaded to ensure data exists
                        log::info!("[GpuState::update_chunk_renderer] 🔄 Force-loading chunk {:?} to ensure data exists", chunk_pos);
                        if let Err(e) = self.world.load_chunk(chunk_pos) {
                            log::warn!("[GpuState::update_chunk_renderer] Force load failed for chunk {:?}: {:?}", chunk_pos, e);
                        }
                    }

                    // Allocate a buffer for this chunk
                    let buffer_index = if let Some(&existing_index) =
                        allocator.allocated_buffers.get(&chunk_pos)
                    {
                        existing_index
                    } else if let Some(new_index) = allocator.free_buffers.pop() {
                        allocator.allocated_buffers.insert(chunk_pos, new_index);
                        new_index
                    } else {
                        log::error!(
                            "[GpuState::update_chunk_renderer] No free mesh buffers available!"
                        );
                        continue;
                    };

                    // Create a simple cube mesh in the buffer
                    if buffer_index < self.gpu_meshing.mesh_buffers.len() as u32 {
                        let mesh_buffer = &self.gpu_meshing.mesh_buffers[buffer_index as usize];

                        // Generate actual terrain mesh for this chunk
                        log::info!("[GpuState::update_chunk_renderer] 🔨 Generating CPU mesh for chunk {:?}", chunk_pos);

                        let (terrain_vertices, terrain_indices) =
                            generate_chunk_terrain_mesh(&self.world, chunk_pos, chunk_size);

                        // Check if we generated any geometry
                        if terrain_vertices.is_empty() {
                            log::warn!("[GpuState::update_chunk_renderer] No terrain geometry generated for chunk {:?}", chunk_pos);
                            continue;
                        }

                        // Upload to GPU
                        self.queue.write_buffer(
                            &mesh_buffer.vertices,
                            0,
                            bytemuck::cast_slice(&terrain_vertices),
                        );
                        self.queue.write_buffer(
                            &mesh_buffer.indices,
                            0,
                            bytemuck::cast_slice(&terrain_indices),
                        );

                        // Create render object
                        let render_object = crate::renderer::gpu_driven::RenderObject {
                            position: cgmath::Vector3::new(
                                (chunk_pos.x * chunk_size as i32) as f32,
                                (chunk_pos.y * chunk_size as i32) as f32,
                                (chunk_pos.z * chunk_size as i32) as f32,
                            ),
                            scale: 1.0,
                            color: [0.3, 0.7, 0.3, 1.0], // Green for terrain
                            bounding_radius: (chunk_size as f32 * 1.732) / 2.0,
                            mesh_id: buffer_index,
                            material_id: 0,
                            index_count: Some(terrain_indices.len() as u32), // Pass actual index count
                        };

                        render_objects.push(render_object);
                        self.dirty_chunks.remove(&chunk_pos);
                        self.chunks_with_meshes.insert(chunk_pos);
                        self.chunk_to_buffer_index.insert(chunk_pos, buffer_index);

                        // Store index count for this mesh
                        self.chunk_index_counts
                            .insert(chunk_pos, terrain_indices.len() as u32);

                        log::info!("[GpuState::update_chunk_renderer] Created CPU mesh for chunk {:?}, buffer_index: {}, indices: {}",
                                  chunk_pos, buffer_index, terrain_indices.len());
                    }
                }
            }
        }

        // Create render objects for all chunks with valid meshes

        for (chunk_pos, _) in self.world.iter_loaded_chunks() {
            if self.chunks_with_meshes.contains(&chunk_pos) {
                // Look up the buffer index for this chunk
                if let Some(&buffer_index) = self.chunk_to_buffer_index.get(&chunk_pos) {
                    let mesh_id = buffer_index;

                    // Create render object for this chunk
                    let world_pos = cgmath::Vector3::new(
                        (chunk_pos.x * chunk_size as i32) as f32,
                        (chunk_pos.y * chunk_size as i32) as f32,
                        (chunk_pos.z * chunk_size as i32) as f32,
                    );

                    let render_object =
                        crate::renderer::gpu_driven::gpu_driven_renderer::RenderObject {
                            position: world_pos,
                            scale: 1.0,
                            color: [1.0, 1.0, 1.0, 1.0],
                            bounding_radius: (chunk_size as f32 * 1.732) / 2.0, // sqrt(3) * chunk_size / 2
                            mesh_id,
                            material_id: 0,
                            index_count: self.chunk_index_counts.get(&chunk_pos).copied(), // Look up stored index count
                        };

                    render_objects.push(render_object);
                } else {
                    log::warn!("[GpuState::update_chunk_renderer] Chunk {:?} has mesh but no buffer index mapping", chunk_pos);
                }
            }
        }

        // Submit all render objects to GPU-driven renderer
        let render_object_count = render_objects.len() as u32;

        // Track submission metrics
        if render_object_count > 0 {
            log::debug!(
                "[GpuState::update_chunk_renderer] Submitting {} render objects (chunks with meshes: {}, loaded chunks: {})",
                render_object_count,
                self.chunks_with_meshes.len(),
                self.world.chunk_manager().loaded_chunk_count()
            );

            // Clear instances before submitting new objects
            // This ensures we're rebuilding the scene with the current set of chunks
            self.chunk_renderer.clear_instances();

            self.chunk_renderer.submit_objects(&render_objects);
            self.total_objects_submitted += render_object_count as u64;
            self.last_submission_time = std::time::Instant::now();

            // CRITICAL: Upload instance data to GPU after submission
            self.chunk_renderer.upload_instances(&self.queue);

            // Reset counter if we had objects
            if self.frames_without_objects > 0 {
                log::info!(
                    "[GpuState::update_chunk_renderer] Resumed object submission after {} frames",
                    self.frames_without_objects
                );
            }
            self.frames_without_objects = 0;
        } else {
            // No objects submitted
            self.frames_without_objects += 1;

            // Log warnings at different thresholds
            if self.frames_without_objects == 60 {
                log::warn!(
                    "[GpuState::update_chunk_renderer] No render objects submitted for 60 frames (1 second). \
                    Chunks with meshes: {}, loaded chunks: {}, dirty chunks: {}",
                    self.chunks_with_meshes.len(),
                    self.world.chunk_manager().loaded_chunk_count(),
                    self.dirty_chunks.len()
                );
            } else if self.frames_without_objects == 300 {
                log::error!(
                    "[GpuState::update_chunk_renderer] No render objects submitted for 300 frames (5 seconds)! \
                    This indicates a problem with chunk meshing or world loading. \
                    Total objects ever submitted: {}",
                    self.total_objects_submitted
                );

                // Log diagnostic information
                self.log_render_diagnostics();
            } else if self.frames_without_objects % 600 == 0 {
                // Every 10 seconds after that
                log::error!(
                    "[GpuState::update_chunk_renderer] Still no render objects after {} frames ({} seconds)",
                    self.frames_without_objects,
                    self.frames_without_objects / 60
                );
                self.log_render_diagnostics();
            }
        }

        // Track changes in submission count
        if render_object_count != self.last_render_object_count {
            let delta = render_object_count as i32 - self.last_render_object_count as i32;
            log::info!(
                "[GpuState::update_chunk_renderer] Render object count changed: {} → {} (delta: {})",
                self.last_render_object_count,
                render_object_count,
                if delta > 0 { format!("+{}", delta) } else { delta.to_string() }
            );
        }
        self.last_render_object_count = render_object_count;

        // Build GPU commands
        self.chunk_renderer.build_commands();

        // Final verification of submission state
        if render_object_count > 0 && self.frames_rendered % 60 == 0 {
            let stats = self.chunk_renderer.stats();
            if stats.objects_rejected > 0 {
                log::warn!(
                    "[GpuState::update_chunk_renderer] Instance buffer may be full: {} objects rejected",
                    stats.objects_rejected
                );
            }
        }

        // Note: Actual rendering happens in the render() method
    }

    /// Log detailed diagnostic information about render state
    fn log_render_diagnostics(&self) {
        log::error!("[GpuState] === RENDER DIAGNOSTICS ===");
        log::error!(
            "[GpuState] Chunks with meshes: {}",
            self.chunks_with_meshes.len()
        );
        log::error!(
            "[GpuState] Loaded chunks: {}",
            self.world.chunk_manager().loaded_chunk_count()
        );
        log::error!("[GpuState] Dirty chunks: {}", self.dirty_chunks.len());
        log::error!(
            "[GpuState] Total objects submitted (lifetime): {}",
            self.total_objects_submitted
        );
        log::error!(
            "[GpuState] Frames without objects: {}",
            self.frames_without_objects
        );
        log::error!("[GpuState] Camera position: {:?}", self.camera.position);
        log::error!(
            "[GpuState] Time since last submission: {:.1}s",
            self.last_submission_time.elapsed().as_secs_f32()
        );

        // Get renderer stats
        let stats = self.chunk_renderer.stats();
        log::error!(
            "[GpuState] Renderer - Objects submitted: {}, drawn: {}, instances: {}, rejected: {}",
            stats.objects_submitted,
            stats.objects_drawn,
            stats.instances_added,
            stats.objects_rejected
        );

        // Log chunk positions if there are any
        if self.chunks_with_meshes.len() > 0 && self.chunks_with_meshes.len() <= 10 {
            log::error!(
                "[GpuState] Chunks with meshes: {:?}",
                self.chunks_with_meshes
            );
        }

        // Check if renderer pipeline is available
        if !self.chunk_renderer.is_available() {
            log::error!("[GpuState] WARNING: GPU-driven renderer is not available!");
        }

        // Check for sync issues
        let loaded_count = self.world.chunk_manager().loaded_chunk_count();
        let mesh_count = self.chunks_with_meshes.len();
        if loaded_count > 0 && mesh_count == 0 {
            log::error!(
                "[GpuState] WARNING: {} chunks loaded but no meshes generated!",
                loaded_count
            );
        }

        log::error!("[GpuState] === END DIAGNOSTICS ===");
    }

    fn process_input(
        &mut self,
        input: &InputState,
        delta_time: f32,
        active_block: BlockId,
    ) -> (Option<(VoxelPos, BlockId)>, Option<VoxelPos>) {
        // Get player body for movement
        log::debug!("[process_input] Player entity ID: {}", self.player_entity);
        if let Some(physics_world) = &mut self.physics_world {
            if let Some(body) = physics_world.get_body_mut(self.player_entity) {
                log::debug!(
                    "[process_input] Body position before: [{:.2}, {:.2}, {:.2}]",
                    body.position[0],
                    body.position[1],
                    body.position[2]
                );
                log::debug!(
                    "[process_input] Body velocity before: [{:.2}, {:.2}, {:.2}]",
                    body.velocity[0],
                    body.velocity[1],
                    body.velocity[2]
                );
                // Calculate movement direction based on camera yaw
                let yaw_rad = self.camera.yaw_radians;
                let forward = Vector3::new(yaw_rad.cos(), 0.0, yaw_rad.sin());
                let right = Vector3::new(yaw_rad.sin(), 0.0, -yaw_rad.cos());

                let mut move_dir = Vector3::new(0.0, 0.0, 0.0);

                // Movement input
                if input.is_key_pressed(KeyCode::KeyW) {
                    log::debug!("[process_input] W key pressed!");
                    move_dir += forward;
                }
                if input.is_key_pressed(KeyCode::KeyS) {
                    log::debug!("[process_input] S key pressed!");
                    move_dir -= forward;
                }
                if input.is_key_pressed(KeyCode::KeyA) {
                    log::debug!("[process_input] A key pressed!");
                    move_dir -= right;
                }
                if input.is_key_pressed(KeyCode::KeyD) {
                    log::debug!("[process_input] D key pressed!");
                    move_dir += right;
                }

                // Normalize diagonal movement
                if move_dir.magnitude() > 0.0 {
                    move_dir = move_dir.normalize();
                }

                // Check player state flags
                let is_grounded = (body.flags & PhysicsFlags::GROUNDED) != 0;
                let is_in_water = (body.flags & PhysicsFlags::IN_WATER) != 0;
                let is_on_ladder = (body.flags & PhysicsFlags::ON_LADDER) != 0;

                // Determine movement speed based on state
                let mut move_speed = 4.3; // Normal walking speed
                if !is_in_water && !is_on_ladder {
                    if input.is_key_pressed(KeyCode::ShiftLeft) && is_grounded {
                        move_speed = 5.6; // Sprint speed
                    } else if input.is_key_pressed(KeyCode::ControlLeft) {
                        move_speed = 1.3; // Crouch speed
                    } else if !is_grounded {
                        // Allow air movement but at reduced speed for better control
                        move_speed = 2.0; // Air movement speed
                    }
                } else if is_in_water {
                    move_speed = 2.0; // Swimming speed
                }

                // Apply horizontal movement
                let horizontal_vel = move_dir * move_speed;
                body.velocity[0] = horizontal_vel.x;
                body.velocity[2] = horizontal_vel.z;

                log::debug!(
                    "[process_input] Move direction: [{:.2}, {:.2}, {:.2}], speed: {:.2}",
                    move_dir.x,
                    move_dir.y,
                    move_dir.z,
                    move_speed
                );
                log::debug!(
                    "[process_input] Body velocity after: [{:.2}, {:.2}, {:.2}]",
                    body.velocity[0],
                    body.velocity[1],
                    body.velocity[2]
                );

                // Provide helpful movement state feedback
                if move_dir.magnitude() > 0.0 {
                    log::debug!("[Movement] Player moving: grounded={}, in_water={}, on_ladder={}, speed={:.1}",
                               is_grounded, is_in_water, is_on_ladder, move_speed);
                } else {
                    static mut MOVEMENT_HELP_COOLDOWN: f32 = 0.0;
                    // SAFETY: Static mut access is safe here because:
                    // - This is only used for UI cooldown timing
                    // - Single-threaded access pattern (render loop)
                    // - Only modified during movement handling
                    // - Race conditions would only affect help message timing
                    unsafe {
                        MOVEMENT_HELP_COOLDOWN -= delta_time;
                        if MOVEMENT_HELP_COOLDOWN <= 0.0 {
                            log::info!("[Movement] Use WASD to move, Space to jump, Shift to sprint, Ctrl to crouch");
                            MOVEMENT_HELP_COOLDOWN = 10.0; // Show help every 10 seconds when not moving
                        }
                    }
                }

                // Handle vertical movement
                if is_on_ladder {
                    // Ladder climbing
                    if input.is_key_pressed(KeyCode::KeyW) {
                        body.velocity[1] = 3.0; // Climb up
                    } else if input.is_key_pressed(KeyCode::KeyS) {
                        body.velocity[1] = -3.0; // Climb down
                    } else {
                        body.velocity[1] = 0.0; // Stay in place on ladder
                    }
                } else if input.is_key_pressed(KeyCode::Space) {
                    if is_in_water {
                        // Swim up
                        body.velocity[1] = 4.0;
                    } else if is_grounded {
                        // Jump
                        body.velocity[1] = 8.5; // Jump velocity
                    }
                } else if is_in_water && input.is_key_pressed(KeyCode::ControlLeft) {
                    // Swim down
                    body.velocity[1] = -2.0;
                }
            } else {
                log::error!(
                    "[process_input] Failed to get player body! Entity ID: {}",
                    self.player_entity
                );
            }
        } else {
            log::error!(
                "[process_input] GPU physics world not available - cannot process player movement"
            );
        }

        // Mouse look - only process if cursor is locked
        if input.is_cursor_locked() {
            let (dx, dy) = input.get_mouse_delta();
            let sensitivity = 0.5;
            self.camera = camera_rotate(
                &self.camera,
                dx * sensitivity * std::f32::consts::PI / 180.0,
                -dy * sensitivity * std::f32::consts::PI / 180.0,
            );
        } else {
            // Provide helpful feedback if cursor is not locked
            static mut CURSOR_WARNING_COOLDOWN: f32 = 0.0;
            // SAFETY: Static mut access is safe here because:
            // - This is only used for UI warning timing
            // - Single-threaded access pattern (render loop)
            // - Only modified during cursor handling
            // - Race conditions would only affect warning message timing
            unsafe {
                CURSOR_WARNING_COOLDOWN -= delta_time;
                if CURSOR_WARNING_COOLDOWN <= 0.0 {
                    log::info!("[Movement] Click in the window or press Escape to lock cursor for mouse look");
                    CURSOR_WARNING_COOLDOWN = 5.0; // Show message every 5 seconds
                }
            }
        }

        // Ray casting for block selection
        let ray = Ray::new(
            Point3::new(
                self.camera.position[0],
                self.camera.position[1],
                self.camera.position[2],
            ),
            calculate_forward_vector(&self.camera),
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
                        if let Err(e) = self.world.set_block(hit.position, BlockId::AIR) {
                            log::error!("Failed to break block: {}", e);
                        }
                        broke_block = Some((hit.position, broken_block_id));
                        self.breaking_block = None;
                        self.breaking_progress = 0.0;

                        // Mark chunk as dirty
                        let chunk_pos = crate::world::voxel_to_chunk_pos(
                            hit.position,
                            self.world.config().chunk_size,
                        );
                        self.dirty_chunks.insert(chunk_pos);
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
                    if let Err(e) = self.world.set_block(place_pos, active_block) {
                        log::error!("Failed to place block: {}", e);
                    }
                    placed_block = Some(place_pos);
                    // Reset breaking progress when placing
                    self.breaking_block = None;
                    self.breaking_progress = 0.0;

                    // Mark chunk as dirty
                    let chunk_pos =
                        crate::world::voxel_to_chunk_pos(place_pos, self.world.config().chunk_size);
                    self.dirty_chunks.insert(chunk_pos);
                }
            }
        }

        (broke_block, placed_block)
    }

    fn render(&mut self, delta_time: f32) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.error_recovery
                .create_safe_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // Track frames rendered
        self.frames_rendered += 1;

        // Log time to first frame
        if self.frames_rendered == 1 {
            let elapsed = self.init_time.elapsed();
            log::info!(
                "[GpuState::render] First frame rendered in {:.2}ms",
                elapsed.as_millis()
            );
        }

        // Track render timing
        let render_start = std::time::Instant::now();

        // Execute GPU culling pass before main render pass
        // Following DOP principles - separate data transformation phases
        let encoder_ref = match encoder.encoder() {
            Ok(enc) => enc,
            Err(e) => {
                log::error!("[GpuState::render] Failed to get encoder: {:?}", e);
                return Err(wgpu::SurfaceError::Lost);
            }
        };
        self.chunk_renderer.execute_culling(encoder_ref);

        {
            let encoder_ref = match encoder.encoder() {
                Ok(enc) => enc,
                Err(e) => {
                    log::error!(
                        "[GpuState::render] Failed to get encoder for render pass: {:?}",
                        e
                    );
                    return Err(wgpu::SurfaceError::Lost);
                }
            };
            let mut render_pass = encoder_ref.begin_render_pass(&wgpu::RenderPassDescriptor {
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

            // Execute GPU-driven rendering draw calls
            self.chunk_renderer
                .render_draw(&mut render_pass, &self.camera_bind_group);

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

        // Update stats after rendering
        self.chunk_renderer.update_stats(render_start);

        // Get stats for logging
        let stats = self.chunk_renderer.stats();
        if stats.objects_drawn > 0 && !self.first_chunks_loaded {
            self.first_chunks_loaded = true;
            log::info!(
                "[GpuState::render] First chunks rendered after {} frames",
                self.frames_rendered
            );

            // Verify spawn position now that chunks are loaded
            let camera_pos_point3 = Point3::new(
                self.camera.position[0],
                self.camera.position[1],
                self.camera.position[2],
            );
            // Use find_safe_spawn to verify the position is still valid
            let adjusted_pos = SpawnFinder::find_safe_spawn(
                &self.world,
                camera_pos_point3.x,
                camera_pos_point3.z,
                5,
            )
            .unwrap_or(camera_pos_point3);
            if adjusted_pos != camera_pos_point3 {
                log::info!(
                    "[GpuState::render] Adjusting spawn position from {:?} to {:?}",
                    self.camera.position,
                    adjusted_pos
                );
                self.camera.position = [adjusted_pos.x, adjusted_pos.y, adjusted_pos.z];

                // Update physics entity position
                if let Some(ref mut physics_world) = self.physics_world {
                    physics_world.set_position(self.player_entity, adjusted_pos);
                    log::info!("[GpuState::render] Updated physics entity position");
                }

                // Update camera uniform
                self.camera_uniform.update_view_proj_data(&self.camera);
                self.queue.write_buffer(
                    &self.camera_buffer,
                    0,
                    bytemuck::cast_slice(&[self.camera_uniform]),
                );
            }

            // Debug what blocks are around spawn
            SpawnFinder::debug_blocks_at_position(
                &self.world,
                Point3::new(
                    self.camera.position[0],
                    self.camera.position[1],
                    self.camera.position[2],
                ),
            );
        } else if stats.objects_drawn == 0 {
            // Log more frequently in the first few seconds
            if self.frames_rendered <= 180 && self.frames_rendered % 20 == 0 {
                log::warn!("[GpuState::render] No chunks rendered after {} frames (objects submitted: {}, world chunks: {}, chunks with meshes: {})",
                         self.frames_rendered,
                         stats.objects_submitted,
                         self.world.chunk_manager().loaded_chunk_count(),
                         self.chunks_with_meshes.len());
            } else if self.frames_rendered % 60 == 0 {
                // Log every second after initial period
                log::warn!("[GpuState::render] No chunks rendered after {} frames (objects submitted: {}, world chunks: {}, chunks with meshes: {})",
                         self.frames_rendered,
                         stats.objects_submitted,
                         self.world.chunk_manager().loaded_chunk_count(),
                         self.chunks_with_meshes.len());
            }
        }

        // Submit render commands with error recovery
        let commands = match encoder.finish() {
            Ok(cmd) => cmd,
            Err(e) => {
                log::error!("[GpuState::render] Failed to finish encoder: {:?}", e);
                return Err(wgpu::SurfaceError::Lost);
            }
        };

        match self.error_recovery.submit_with_recovery(vec![commands]) {
            Ok(_) => {}
            Err(e) => {
                log::error!("[GpuState::render] Failed to submit commands: {:?}", e);
                return Err(wgpu::SurfaceError::Lost);
            }
        }

        output.present();

        // Update frame timing for frame rate limiting
        self.last_frame_time = Some(std::time::Instant::now());

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
    let (width, height, was_clamped) =
        validate_texture_dimensions(config.width, config.height, max_texture_dimension);

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
        width,
        height,
        max_texture_dimension
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

pub async fn run_app<G: GameData + 'static>(
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

    log::info!("[gpu_state::run_app] Creating GPU state with game block registration...");
    let mut gpu_state = match GpuState::new_with_game(window.clone(), Some(&mut game), config).await
    {
        Ok(state) => {
            log::info!("[gpu_state::run_app] GPU state created successfully");
            state
        }
        Err(e) => {
            log::error!("[gpu_state::run_app] GPU state creation failed: {}", e);
            return Err(e);
        }
    };

    // Custom world generator is already handled in GpuState::new_with_game
    // via the config.world_generator_factory mechanism

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
            eprintln!(
                "Initial cursor lock failed: {:?}. Trying confined mode...",
                e
            );
            gpu_state
                .window
                .set_cursor_grab(CursorGrabMode::Confined)
                .ok();
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
                        // Re-lock cursor when clicking back into the window
                        if *state == winit::event::ElementState::Pressed && !input_state.is_cursor_locked() {
                            // Lock cursor on any mouse button press when not locked
                            input_state.set_cursor_locked(true);
                            input_state.reset_mouse_tracking(); // Reset mouse tracking to avoid jumps
                            match gpu_state.window.set_cursor_grab(CursorGrabMode::Locked) {
                                Ok(_) => {
                                    gpu_state.window.set_cursor_visible(false);
                                }
                                Err(e) => {
                                    eprintln!("Failed to re-lock cursor: {:?}. Trying confined mode...", e);
                                    gpu_state.window.set_cursor_grab(CursorGrabMode::Confined).ok();
                                    gpu_state.window.set_cursor_visible(false);
                                }
                            }
                        }
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
                        let active_block = get_active_block_from_game(&game);
                        let (broken_block_info, placed_block_pos) = gpu_state.process_input(&input_state, delta_time, active_block);

                        // Log camera info periodically for debugging
                        if gpu_state.frames_rendered % 60 == 0 {
                            log::info!("[render loop] Frame {}: Camera pos: ({:.2}, {:.2}, {:.2}), yaw: {:.2}, pitch: {:.2}",
                                gpu_state.frames_rendered,
                                gpu_state.camera.position[0],
                                gpu_state.camera.position[1],
                                gpu_state.camera.position[2],
                                gpu_state.camera.yaw_radians,
                                gpu_state.camera.pitch_radians);
                        }

                        // Update physics
                        log::trace!("[render loop] Updating physics with delta_time: {:.4}", delta_time);
                        if let Some(ref mut physics_world) = gpu_state.physics_world {
                            physics_world.update(&gpu_state.world, delta_time);
                        }

                        // Sync camera position with player physics body
                        log::trace!("[render loop] Syncing camera with player entity {}", gpu_state.player_entity);
                        if let Some(ref physics_world) = gpu_state.physics_world {
                            if let Some(body) = physics_world.get_body(gpu_state.player_entity) {
                                log::debug!("[render loop] Physics body position: [{:.2}, {:.2}, {:.2}]", body.position[0], body.position[1], body.position[2]);
                                let player_pos = Point3::new(
                                    body.position[0],
                                    body.position[1],
                                    body.position[2],
                                );

                                // Camera at eye level (0.72m offset from body center)
                                gpu_state.camera.position = [
                                    player_pos.x,
                                    player_pos.y + 0.72,
                                    player_pos.z
                                ];
                                log::debug!("[render loop] Camera position updated to: [{:.2}, {:.2}, {:.2}]", gpu_state.camera.position[0], gpu_state.camera.position[1], gpu_state.camera.position[2]);
                            }
                        }

                        // Update loaded chunks based on player position
                        // Always update world to ensure chunks are loaded and unloaded properly
                        if gpu_state.frames_rendered <= 10 || gpu_state.frames_rendered % 60 == 0 {
                            log::info!("[render loop] World update #{} at camera position: {:?} (loaded chunks: {})",
                                     gpu_state.frames_rendered,
                                     gpu_state.camera.position,
                                     gpu_state.world.chunk_manager().loaded_chunk_count());
                        }

                        // Ensure camera chunk is loaded before world update
                        let camera_pos = Point3::new(gpu_state.camera.position[0], gpu_state.camera.position[1], gpu_state.camera.position[2]);
                        let camera_chunk_loaded = gpu_state.world.ensure_camera_chunk_loaded(camera_pos);
                        if !camera_chunk_loaded {
                            log::warn!("[render loop] Camera chunk still being generated at position: {:?}", camera_pos);
                        }

                        // Get current loaded chunks before update
                        let chunks_before: std::collections::HashSet<_> = gpu_state.world.iter_loaded_chunks()
                            .map(|(pos, _)| pos)
                            .collect();

                        gpu_state.world.update(camera_pos);

                        // Mark any newly loaded chunks as dirty so they get meshed
                        let chunks_after: std::collections::HashSet<_> = gpu_state.world.iter_loaded_chunks()
                            .map(|(pos, _)| pos)
                            .collect();

                        for chunk_pos in chunks_after.difference(&chunks_before) {
                            log::debug!("[render loop] New chunk loaded at {:?}, marking as dirty", chunk_pos);
                            gpu_state.dirty_chunks.insert(*chunk_pos);
                        }

                        // Periodic sync check
                        if gpu_state.frames_rendered % 300 == 0 && gpu_state.frames_rendered > 0 {
                            let loaded_chunks = gpu_state.world.chunk_manager().loaded_chunk_count();
                            let chunks_with_meshes = gpu_state.chunks_with_meshes.len();
                            let render_objects = gpu_state.last_render_object_count;

                            log::info!(
                                "[render loop] Periodic sync check - Loaded chunks: {}, Chunks with meshes: {}, Render objects: {}",
                                loaded_chunks,
                                chunks_with_meshes,
                                render_objects
                            );

                            // Detect sync issues
                            if loaded_chunks > 0 && render_objects == 0 {
                                log::warn!(
                                    "[render loop] Sync issue detected: {} chunks loaded but no render objects!",
                                    loaded_chunks
                                );
                            }
                        }

                        // Update day/night cycle
                        update_day_night_cycle(&mut gpu_state.day_night_cycle, delta_time);

                        // Update block lighting if blocks were changed
                        if let Some((pos, block_id)) = broken_block_info {
                            // A block was broken - check if it was a light source
                            if let Some(block) = gpu_state.block_registry.get_block(block_id) {
                                if block.get_light_emission() > 0 {
                                    // Removed a light source - queue for GPU processing
                                    if let Some(ref light_propagator) = gpu_state.light_propagator {
                                        let update = LightUpdate { pos, light_type: LightType::Block, level: block.get_light_emission(), is_removal: true };
                                        light_propagator.add_update(update);
                                    }
                                }
                            }
                            // Update skylight column
                            // TODO: Port skylight updates
                        }

                        if let Some(place_pos) = placed_block_pos {
                            if let Some(block) = gpu_state.block_registry.get_block(active_block) {
                                if block.get_light_emission() > 0 {
                                    // Placed a light source - queue for GPU processing
                                    if let Some(ref light_propagator) = gpu_state.light_propagator {
                                        let update = LightUpdate { pos: place_pos, light_type: LightType::Block, level: block.get_light_emission(), is_removal: false };
                                        light_propagator.add_update(update);
                                    }
                                }
                            }
                            // Update skylight column
                            // TODO: Port skylight updates
                        }

                        // Process light propagation if needed
                        if broken_block_info.is_some() || placed_block_pos.is_some() {
                            if let Some(ref light_propagator) = gpu_state.light_propagator {
                                // Process GPU lighting updates
                                if let Err(e) = light_propagator.process_updates() {
                                    log::error!("[GPU Lighting] Failed to process light updates: {:?}", e);
                                }
                            }
                        }

                        gpu_state.update_camera();
                        input_state.clear_mouse_delta();

                        // Update async chunk renderer
                        gpu_state.update_chunk_renderer(&input_state);

                        // Log chunk renderer state for first few frames and periodically
                        if gpu_state.frames_rendered <= 10 && gpu_state.frames_rendered % 2 == 0 {
                            let stats = gpu_state.chunk_renderer.stats();
                            log::info!("[render loop] Frame {}: chunk renderer has {} objects submitted, {} drawn",
                                     gpu_state.frames_rendered,
                                     stats.objects_submitted,
                                     stats.objects_drawn);
                        } else if gpu_state.frames_rendered % 600 == 0 {
                            // Log every 10 seconds
                            let stats = gpu_state.chunk_renderer.stats();
                            let time_since_submission = gpu_state.last_submission_time.elapsed();
                            log::info!(
                                "[render loop] Frame {}: {} objects submitted, {} drawn, last submission: {:.1}s ago",
                                gpu_state.frames_rendered,
                                stats.objects_submitted,
                                stats.objects_drawn,
                                time_since_submission.as_secs_f32()
                            );
                        }

                        // Update game with context
                        let mut ctx = GameContext {
                            world: &mut gpu_state.world,
                            registry: &gpu_state.block_registry,
                            camera: &gpu_state.camera,
                            input: &input_state,
                            selected_block: gpu_state.selected_block.clone(),
                        };
                        update_game(&mut game, &mut ctx, delta_time);

                        // Handle block callbacks
                        if let Some((pos, block_id)) = broken_block_info {
                            handle_block_break(&mut game, pos, block_id);
                        }
                        if let Some(place_pos) = placed_block_pos {
                            handle_block_place(&mut game, place_pos, active_block);
                        }

                        // Render
                        match gpu_state.render(delta_time) {
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
                    // Only request redraw if enough time has passed (FPS target)
                    let now = std::time::Instant::now();
                    let target_frametime = std::time::Duration::from_secs_f64(1.0 / gameplay::TARGET_FPS as f64);

                    if let Some(last_frame) = gpu_state.last_frame_time {
                        let elapsed = now.duration_since(last_frame);
                        if elapsed >= target_frametime {
                            gpu_state.window.request_redraw();
                        }
                    } else {
                        gpu_state.window.request_redraw();
                    }
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

    log::info!(
        "[GPU Tier] Analyzing GPU: {} (vendor: 0x{:04x})",
        info.name,
        info.vendor
    );

    // For D3D12-wrapped GPUs, vendor ID might be 0x0000, so also check name
    let is_nvidia = info.vendor == NVIDIA
        || name_lower.contains("nvidia")
        || name_lower.contains("geforce")
        || name_lower.contains("rtx")
        || name_lower.contains("gtx");
    let is_amd = info.vendor == AMD
        || name_lower.contains("amd")
        || name_lower.contains("radeon")
        || name_lower.contains("rx ");
    let is_intel =
        info.vendor == INTEL || (name_lower.contains("intel") && !name_lower.contains("nvidia"));
    let is_apple = info.vendor == APPLE || name_lower.contains("apple");

    if is_nvidia {
        log::info!("[GPU Tier] Detected NVIDIA GPU (vendor check or name match)");
        // NVIDIA GPU detection
        if name_lower.contains("rtx 40")
            || name_lower.contains("rtx 4080")
            || name_lower.contains("rtx 4090")
        {
            log::info!("[GPU Tier] Detected high-end NVIDIA RTX 40 series");
            GpuTier::HighEnd
        } else if name_lower.contains("rtx 4070")
            || name_lower.contains("rtx 4060")
            || name_lower.contains("rtx 30")
            || name_lower.contains("rtx 3070")
            || name_lower.contains("rtx 3080")
            || name_lower.contains("rtx 3090")
        {
            // RTX 4060 Ti has excellent capabilities despite mid-range positioning
            if name_lower.contains("rtx 4060 ti") && has_high_texture_support {
                log::info!("[GPU Tier] Detected NVIDIA RTX 4060 Ti - using high-end profile");
                GpuTier::HighEnd
            } else {
                log::info!("[GPU Tier] Detected mid-range NVIDIA RTX");
                GpuTier::MidRange
            }
        } else if name_lower.contains("rtx")
            || name_lower.contains("gtx 16")
            || name_lower.contains("gtx 20")
        {
            log::info!("[GPU Tier] Detected entry-level NVIDIA GPU");
            GpuTier::Entry
        } else if has_high_texture_support && has_large_buffers {
            // Unknown NVIDIA GPU but has good capabilities
            GpuTier::MidRange
        } else {
            GpuTier::LowEnd
        }
    } else if is_amd {
        log::info!("[GPU Tier] Detected AMD GPU");
        // AMD GPU detection
        if name_lower.contains("rx 7900")
            || name_lower.contains("rx 7800")
            || name_lower.contains("rx 6900")
            || name_lower.contains("rx 6800")
        {
            log::info!("[GPU Tier] Detected high-end AMD GPU");
            GpuTier::HighEnd
        } else if name_lower.contains("rx 7700")
            || name_lower.contains("rx 7600")
            || name_lower.contains("rx 6700")
            || name_lower.contains("rx 6600")
            || name_lower.contains("rx 5700")
        {
            log::info!("[GPU Tier] Detected mid-range AMD GPU");
            GpuTier::MidRange
        } else if name_lower.contains("rx") && has_high_texture_support {
            GpuTier::Entry
        } else {
            GpuTier::LowEnd
        }
    } else if is_intel {
        log::info!("[GPU Tier] Detected Intel GPU");
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
    } else if is_apple {
        log::info!("[GPU Tier] Detected Apple GPU");
        // Apple Silicon detection
        if name_lower.contains("m2 pro")
            || name_lower.contains("m2 max")
            || name_lower.contains("m3 pro")
            || name_lower.contains("m3 max")
            || name_lower.contains("m1 pro")
            || name_lower.contains("m1 max")
        {
            log::info!("[GPU Tier] Detected high-end Apple Silicon");
            GpuTier::HighEnd
        } else if name_lower.contains("m1")
            || name_lower.contains("m2")
            || name_lower.contains("m3")
        {
            log::info!("[GPU Tier] Detected Apple Silicon");
            GpuTier::MidRange
        } else {
            GpuTier::Entry
        }
    } else {
        // Unknown vendor - use capabilities to determine tier
        log::info!("[GPU Tier] Unknown vendor, analyzing capabilities...");
        if has_high_texture_support && has_large_buffers && has_many_bind_groups {
            log::info!("[GPU Tier] Unknown GPU with high-end capabilities");
            GpuTier::MidRange
        } else if limits.max_texture_dimension_2d >= 8192
            && limits.max_buffer_size >= 512 * 1024 * 1024
        {
            log::info!("[GPU Tier] Unknown GPU with mid-range capabilities");
            GpuTier::Entry
        } else {
            log::info!("[GPU Tier] Unknown GPU with limited capabilities");
            GpuTier::LowEnd
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
            limits.max_texture_dimension_1d = limits
                .max_texture_dimension_1d
                .min(hardware_limits.max_texture_dimension_1d);
            limits.max_texture_dimension_2d = limits
                .max_texture_dimension_2d
                .min(hardware_limits.max_texture_dimension_2d);
            limits.max_texture_dimension_3d = limits
                .max_texture_dimension_3d
                .min(hardware_limits.max_texture_dimension_3d);
            limits.max_buffer_size = limits.max_buffer_size.min(hardware_limits.max_buffer_size);
            limits.max_vertex_buffers = limits
                .max_vertex_buffers
                .min(hardware_limits.max_vertex_buffers);
            limits.max_bind_groups = limits.max_bind_groups.min(hardware_limits.max_bind_groups);
            limits.max_vertex_attributes = limits
                .max_vertex_attributes
                .min(hardware_limits.max_vertex_attributes);
            limits.max_uniform_buffer_binding_size = limits
                .max_uniform_buffer_binding_size
                .min(hardware_limits.max_uniform_buffer_binding_size);

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
            limits.max_texture_dimension_1d = limits
                .max_texture_dimension_1d
                .min(hardware_limits.max_texture_dimension_1d);
            limits.max_texture_dimension_2d = limits
                .max_texture_dimension_2d
                .min(hardware_limits.max_texture_dimension_2d);
            limits.max_texture_dimension_3d = limits
                .max_texture_dimension_3d
                .min(hardware_limits.max_texture_dimension_3d);
            limits.max_buffer_size = limits.max_buffer_size.min(hardware_limits.max_buffer_size);
            limits.max_vertex_buffers = limits
                .max_vertex_buffers
                .min(hardware_limits.max_vertex_buffers);
            limits.max_bind_groups = limits.max_bind_groups.min(hardware_limits.max_bind_groups);

            limits
        }
        GpuTier::Entry => {
            log::info!("[GPU Limits] Using entry-level GPU profile");
            // Use downlevel defaults
            let mut limits = wgpu::Limits::downlevel_defaults();

            // Ensure minimum texture size for Hearth Engine
            if hardware_limits.max_texture_dimension_2d >= 4096 {
                limits.max_texture_dimension_2d = 4096;
            }

            // Clamp to hardware
            limits.max_texture_dimension_1d = limits
                .max_texture_dimension_1d
                .min(hardware_limits.max_texture_dimension_1d);
            limits.max_texture_dimension_2d = limits
                .max_texture_dimension_2d
                .min(hardware_limits.max_texture_dimension_2d);
            limits.max_texture_dimension_3d = limits
                .max_texture_dimension_3d
                .min(hardware_limits.max_texture_dimension_3d);
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
            limits.max_texture_dimension_1d = limits
                .max_texture_dimension_1d
                .min(hardware_limits.max_texture_dimension_1d);
            limits.max_texture_dimension_2d = limits
                .max_texture_dimension_2d
                .min(hardware_limits.max_texture_dimension_2d);
            limits.max_texture_dimension_3d = limits
                .max_texture_dimension_3d
                .min(hardware_limits.max_texture_dimension_3d);
            limits.max_buffer_size = limits.max_buffer_size.min(hardware_limits.max_buffer_size);

            limits
        }
    }
}

/// Optimize limits specifically for voxel engine requirements
fn optimize_limits_for_voxel_engine(
    limits: &mut wgpu::Limits,
    hardware_limits: &wgpu::Limits,
    tier: GpuTier,
) {
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
                log::info!(
                    "[GPU Limits] Using 4096x4096 texture atlas support (minimum for quality)"
                );
            } else {
                log::warn!(
                    "[GPU Limits] GPU doesn't support 4096x4096 textures, quality may be reduced"
                );
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
                log::info!(
                    "[GPU Limits] Using {}MB buffer size",
                    limits.max_buffer_size / 1024 / 1024
                );
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
        log::info!(
            "[GPU Limits] GPU supports 256+ compute workgroup size - good for parallel algorithms"
        );
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
