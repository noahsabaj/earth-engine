// GPU Renderer - Main rendering pipeline
// Implements GPU-driven rendering with single draw call

export class GPURenderer {
    constructor(gpu, worldBuffer, meshGenerator) {
        this.gpu = gpu;
        this.device = gpu.device;
        this.worldBuffer = worldBuffer;
        this.meshGenerator = meshGenerator;
        
        // Pipeline objects
        this.renderPipeline = null;
        this.depthTexture = null;
        this.cameraBuffer = null;
        
        // Bind groups
        this.cameraBindGroup = null;
        this.meshBindGroup = null;
        
        // Stats
        this.frameCount = 0;
        this.lastFrameTime = 0;
        this.fps = 0;
    }
    
    async init() {
        console.log('[Renderer] Initializing GPU renderer...');
        
        // Create depth texture
        this.createDepthTexture();
        
        // Create camera uniform buffer
        this.cameraBuffer = this.device.createBuffer({
            label: 'CameraUniform',
            size: 256, // Padded for alignment
            usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
        });
        
        // Create render pipeline
        await this.createRenderPipeline();
        
        // Create bind groups
        this.createBindGroups();
        
        console.log('[Renderer] GPU renderer initialized');
    }
    
    createDepthTexture() {
        if (this.depthTexture) {
            this.depthTexture.destroy();
        }
        
        this.depthTexture = this.device.createTexture({
            label: 'DepthTexture',
            size: [this.gpu.canvas.width, this.gpu.canvas.height, 1],
            format: 'depth24plus',
            usage: GPUTextureUsage.RENDER_ATTACHMENT,
        });
        
        this.depthTextureView = this.depthTexture.createView();
    }
    
    async createRenderPipeline() {
        // Vertex shader
        const vertexShader = `
            struct CameraUniforms {
                view: mat4x4<f32>,
                projection: mat4x4<f32>,
                viewProjection: mat4x4<f32>,
                position: vec4<f32>,
            }
            
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
            
            fn unpack_color(packed: u32) -> vec4<f32> {
                return vec4<f32>(
                    f32((packed >> 16u) & 0xFFu) / 255.0,
                    f32((packed >> 8u) & 0xFFu) / 255.0,
                    f32(packed & 0xFFu) / 255.0,
                    f32((packed >> 24u) & 0xFFu) / 255.0
                );
            }
            
            @vertex
            fn vs_main(in: VertexInput) -> VertexOutput {
                var out: VertexOutput;
                
                out.world_position = in.position;
                out.clip_position = camera.viewProjection * vec4<f32>(in.position, 1.0);
                out.normal = in.normal;
                out.uv = in.uv;
                out.color = unpack_color(in.color);
                
                // Simple fog based on distance
                let distance = length(camera.position.xyz - in.position);
                out.fog_factor = smoothstep(50.0, 300.0, distance);
                
                return out;
            }
        `;
        
        // Fragment shader
        const fragmentShader = `
            struct CameraUniforms {
                view: mat4x4<f32>,
                projection: mat4x4<f32>,
                viewProjection: mat4x4<f32>,
                position: vec4<f32>,
            }
            
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
                
                // Apply lighting to color
                var final_color = in.color.rgb * lighting;
                
                // Apply fog
                final_color = mix(final_color, FOG_COLOR, in.fog_factor);
                
                return vec4<f32>(final_color, 1.0);
            }
        `;
        
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
        
        this.renderPipeline = await this.gpu.createRenderPipeline(
            vertexShader,
            fragmentShader,
            {
                vertexBuffers: [vertexBufferLayout],
                depthStencil: {
                    depthWriteEnabled: true,
                    depthCompare: 'less',
                    format: 'depth24plus',
                },
                cullMode: 'back',
            }
        );
    }
    
    createBindGroups() {
        // Camera bind group
        this.cameraBindGroupLayout = this.device.createBindGroupLayout({
            label: 'CameraBindGroupLayout',
            entries: [
                {
                    binding: 0,
                    visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
                    buffer: { type: 'uniform' },
                },
            ],
        });
        
        this.cameraBindGroup = this.device.createBindGroup({
            label: 'CameraBindGroup',
            layout: this.cameraBindGroupLayout,
            entries: [
                {
                    binding: 0,
                    resource: { buffer: this.cameraBuffer },
                },
            ],
        });
    }
    
    resize() {
        this.gpu.resize();
        this.createDepthTexture();
    }
    
    render(camera) {
        if (!this.renderPipeline) return;
        
        // Update camera
        this.device.queue.writeBuffer(this.cameraBuffer, 0, camera.toGPUData());
        
        // Get current texture
        const currentTexture = this.gpu.context.getCurrentTexture();
        const view = currentTexture.createView();
        
        // Create command encoder
        const encoder = this.device.createCommandEncoder();
        
        // Render pass
        const renderPass = encoder.beginRenderPass({
            colorAttachments: [{
                view: view,
                clearValue: { r: 0.5, g: 0.8, b: 1.0, a: 1.0 }, // Sky blue
                loadOp: 'clear',
                storeOp: 'store',
            }],
            depthStencilAttachment: {
                view: this.depthTextureView,
                depthClearValue: 1.0,
                depthLoadOp: 'clear',
                depthStoreOp: 'store',
            },
        });
        
        renderPass.setPipeline(this.renderPipeline);
        renderPass.setBindGroup(0, this.cameraBindGroup);
        
        // Bind mesh data
        renderPass.setVertexBuffer(0, this.meshGenerator.vertexBuffer);
        renderPass.setIndexBuffer(this.meshGenerator.indexBuffer, 'uint32');
        
        // Draw the mesh
        if (this.meshGenerator.totalIndices > 0) {
            // Only log occasionally to avoid spam
            if (this.frameCount % 60 === 0) {
                console.log('[Renderer] Drawing', this.meshGenerator.totalIndices, 'indices');
            }
            // Direct draw for debugging
            renderPass.drawIndexed(this.meshGenerator.totalIndices);
            // TODO: Switch back to indirect: renderPass.drawIndexedIndirect(this.meshGenerator.indirectBuffer, 0);
        } else if (this.frameCount % 60 === 0) {
            console.log('[Renderer] No indices to draw!');
        }
        
        renderPass.end();
        
        this.device.queue.submit([encoder.finish()]);
        
        // Update stats
        this.updateStats();
    }
    
    updateStats() {
        this.frameCount++;
        
        const now = performance.now();
        if (now - this.lastFrameTime >= 1000) {
            this.fps = this.frameCount;
            this.frameCount = 0;
            this.lastFrameTime = now;
        }
    }
    
    getStats() {
        return {
            fps: this.fps,
            drawCalls: 1, // Always 1 with GPU-driven rendering!
            vertices: this.meshGenerator.totalVertices,
            triangles: Math.floor(this.meshGenerator.totalIndices / 3),
        };
    }
}