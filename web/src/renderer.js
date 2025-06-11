// Renderer - Pure functions for GPU rendering
// No classes, just data and functions

import { gpuState, createTexture, createRenderPipeline } from './gpu-state.js';
import { cameraState, uploadCameraData } from './camera-state.js';
import { meshState } from './mesh-generation.js';
import { SHADER_SNIPPETS } from './shader-snippets.js';

// Renderer state - pure data
export const rendererState = {
    // Pipeline
    pipeline: null,
    
    // Depth texture
    depthTexture: null,
    depthTextureView: null,
    
    // Bind groups
    bindGroupLayout: null,
    bindGroup: null,
    
    // Stats
    stats: {
        fps: 0,
        frameCount: 0,
        lastFrameTime: 0
    },
    
    initialized: false
};

// Vertex shader code
function createVertexShader() {
    return `
        ${SHADER_SNIPPETS.cameraStruct}
        
        struct VertexInput {
            @location(0) position: vec3<f32>,
            @location(1) normal: vec3<f32>,
            @location(2) uv: vec2<f32>,
            @location(3) @interpolate(flat) color: u32,
        }
        
        struct VertexOutput {
            @builtin(position) clip_position: vec4<f32>,
            @location(0) world_position: vec3<f32>,
            @location(1) normal: vec3<f32>,
            @location(2) uv: vec2<f32>,
            @location(3) color: vec4<f32>,
            @location(4) fog_factor: f32,
        }
        
        @group(0) @binding(0) var<uniform> camera: CameraUniforms;
        
        ${SHADER_SNIPPETS.unpackColor}
        
        @vertex
        fn vs_main(in: VertexInput) -> VertexOutput {
            var out: VertexOutput;
            
            out.world_position = in.position;
            out.clip_position = camera.viewProjection * vec4<f32>(in.position, 1.0);
            out.normal = in.normal;
            out.uv = in.uv;
            out.color = unpack_color(in.color);
            
            // Simple fog
            let distance = length(camera.position.xyz - in.position);
            out.fog_factor = smoothstep(50.0, 300.0, distance);
            
            return out;
        }
    `;
}

// Fragment shader code
function createFragmentShader() {
    return `
        ${SHADER_SNIPPETS.cameraStruct}
        
        struct FragmentInput {
            @location(0) world_position: vec3<f32>,
            @location(1) normal: vec3<f32>,
            @location(2) uv: vec2<f32>,
            @location(3) color: vec4<f32>,
            @location(4) fog_factor: f32,
        }
        
        @group(0) @binding(0) var<uniform> camera: CameraUniforms;
        
        const SUN_DIR = vec3<f32>(0.3, -0.8, 0.5);
        const FOG_COLOR = vec3<f32>(0.5, 0.8, 1.0);
        
        @fragment
        fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
            // Basic lighting
            let ambient = 0.3;
            let diffuse = max(0.0, dot(normalize(in.normal), -normalize(SUN_DIR)));
            let lighting = ambient + diffuse * 0.7;
            
            // Apply lighting
            var final_color = in.color.rgb * lighting;
            
            // Apply fog
            final_color = mix(final_color, FOG_COLOR, in.fog_factor);
            
            return vec4<f32>(final_color, 1.0);
        }
    `;
}

// Create depth texture
export function createDepthTexture() {
    if (rendererState.depthTexture) {
        rendererState.depthTexture.destroy();
    }
    
    rendererState.depthTexture = createTexture(
        gpuState.canvas.width,
        gpuState.canvas.height,
        'depth24plus',
        GPUTextureUsage.RENDER_ATTACHMENT,
        'DepthTexture'
    );
    
    rendererState.depthTextureView = rendererState.depthTexture.createView();
}

// Initialize renderer
export function initializeRenderer() {
    console.log('[Renderer] Initializing...');
    
    // Create depth texture
    createDepthTexture();
    
    // Vertex buffer layout
    const vertexBufferLayout = {
        arrayStride: 36, // 3 floats pos + 3 floats normal + 2 floats uv + 1 u32 color + padding
        attributes: [
            {
                format: 'float32x3',
                offset: 0,
                shaderLocation: 0, // position
            },
            {
                format: 'float32x3',
                offset: 12,
                shaderLocation: 1, // normal
            },
            {
                format: 'float32x2',
                offset: 24,
                shaderLocation: 2, // uv
            },
            {
                format: 'uint32',
                offset: 32,
                shaderLocation: 3, // color
            },
        ],
    };
    
    // Create render pipeline
    rendererState.pipeline = createRenderPipeline(
        createVertexShader(),
        createFragmentShader(),
        {
            label: 'MainRenderPipeline',
            vertexBuffers: [vertexBufferLayout],
            depthStencil: {
                depthWriteEnabled: true,
                depthCompare: 'less',
                format: 'depth24plus',
            },
            cullMode: 'back',
        }
    );
    
    // Create bind group layout
    rendererState.bindGroupLayout = gpuState.device.createBindGroupLayout({
        label: 'RenderBindGroupLayout',
        entries: [
            {
                binding: 0,
                visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
                buffer: { type: 'uniform' },
            },
        ],
    });
    
    // Create bind group
    rendererState.bindGroup = gpuState.device.createBindGroup({
        label: 'RenderBindGroup',
        layout: rendererState.bindGroupLayout,
        entries: [
            {
                binding: 0,
                resource: { buffer: cameraState.buffer },
            },
        ],
    });
    
    rendererState.initialized = true;
    console.log('[Renderer] Initialized');
}

// Render frame
export function renderFrame() {
    if (!rendererState.initialized) return;
    
    // Update camera uniforms
    uploadCameraData();
    
    // Get current texture
    const currentTexture = gpuState.context.getCurrentTexture();
    const view = currentTexture.createView();
    
    // Create command encoder
    const encoder = gpuState.device.createCommandEncoder();
    
    // Begin render pass
    const renderPass = encoder.beginRenderPass({
        colorAttachments: [{
            view: view,
            clearValue: { r: 0.5, g: 0.8, b: 1.0, a: 1.0 }, // Sky blue
            loadOp: 'clear',
            storeOp: 'store',
        }],
        depthStencilAttachment: {
            view: rendererState.depthTextureView,
            depthClearValue: 1.0,
            depthLoadOp: 'clear',
            depthStoreOp: 'store',
        },
    });
    
    renderPass.setPipeline(rendererState.pipeline);
    renderPass.setBindGroup(0, rendererState.bindGroup);
    
    // Bind mesh data
    renderPass.setVertexBuffer(0, meshState.buffers.vertex);
    renderPass.setIndexBuffer(meshState.buffers.index, 'uint32');
    
    // Draw using indirect buffer
    renderPass.drawIndexedIndirect(meshState.buffers.indirect, 0);
    
    renderPass.end();
    
    // Submit
    gpuState.device.queue.submit([encoder.finish()]);
    
    // Update stats
    updateRenderStats();
}

// Update render statistics
function updateRenderStats() {
    rendererState.stats.frameCount++;
    
    const now = performance.now();
    if (now - rendererState.stats.lastFrameTime >= 1000) {
        rendererState.stats.fps = rendererState.stats.frameCount;
        rendererState.stats.frameCount = 0;
        rendererState.stats.lastFrameTime = now;
    }
}

// Resize handler
export function resizeRenderer() {
    createDepthTexture();
}