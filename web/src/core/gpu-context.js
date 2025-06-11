// GPU Context - WebGPU initialization and management
// Direct equivalent of Rust's wgpu context

export class GPUContext {
    constructor(canvas) {
        this.canvas = canvas;
        this.device = null;
        this.queue = null;
        this.context = null;
        this.format = null;
        this.initialized = false;
    }
    
    async init() {
        console.log('[GPU] Initializing WebGPU context...');
        
        // Check WebGPU support
        if (!navigator.gpu) {
            throw new Error('WebGPU not supported in this browser');
        }
        
        // Request adapter - same as Rust
        const adapter = await navigator.gpu.requestAdapter({
            powerPreference: 'high-performance',
            forceFallbackAdapter: false,
        });
        
        if (!adapter) {
            throw new Error('Failed to get WebGPU adapter');
        }
        
        console.log('[GPU] Adapter:', adapter.info);
        
        // Request device with specific features
        this.device = await adapter.requestDevice({
            requiredFeatures: [
                // Add features as needed
            ],
            requiredLimits: {
                maxStorageBufferBindingSize: 1024 * 1024 * 1024, // 1GB
                maxBufferSize: 1024 * 1024 * 1024,
                maxComputeWorkgroupSizeX: 256,
                maxComputeWorkgroupSizeY: 256,
                maxComputeWorkgroupSizeZ: 64,
            }
        });
        
        this.queue = this.device.queue;
        
        // Configure canvas context
        this.context = this.canvas.getContext('webgpu');
        this.format = navigator.gpu.getPreferredCanvasFormat();
        this.presentationFormat = this.format; // Alias for compatibility
        
        this.context.configure({
            device: this.device,
            format: this.format,
            usage: GPUTextureUsage.RENDER_ATTACHMENT,
            alphaMode: 'opaque',
        });
        
        // Set canvas size
        this.resize();
        
        // Handle lost device
        this.device.lost.then((info) => {
            console.error(`WebGPU device was lost: ${info.message}`);
            // Could attempt to reinitialize here
        });
        
        this.initialized = true;
        console.log('[GPU] WebGPU context initialized successfully');
        
        return this;
    }
    
    resize() {
        const dpr = window.devicePixelRatio || 1;
        const width = this.canvas.clientWidth * dpr;
        const height = this.canvas.clientHeight * dpr;
        
        if (this.canvas.width !== width || this.canvas.height !== height) {
            this.canvas.width = width;
            this.canvas.height = height;
            console.log(`[GPU] Canvas resized to ${width}x${height}`);
        }
    }
    
    // Helper to create compute pipeline
    async createComputePipeline(shaderCode, entryPoint = 'main') {
        const shaderModule = this.device.createShaderModule({
            code: shaderCode,
        });
        
        return this.device.createComputePipeline({
            layout: 'auto',
            compute: {
                module: shaderModule,
                entryPoint: entryPoint,
            }
        });
    }
    
    // Helper to create render pipeline
    async createRenderPipeline(vertexShader, fragmentShader, options = {}) {
        const vertexModule = this.device.createShaderModule({ code: vertexShader });
        const fragmentModule = this.device.createShaderModule({ code: fragmentShader });
        
        return this.device.createRenderPipeline({
            layout: 'auto',
            vertex: {
                module: vertexModule,
                entryPoint: options.vertexEntry || 'vs_main',
                buffers: options.vertexBuffers || [],
            },
            fragment: {
                module: fragmentModule,
                entryPoint: options.fragmentEntry || 'fs_main',
                targets: [{
                    format: this.format,
                    blend: options.blend || undefined,
                    writeMask: GPUColorWrite.ALL,
                }],
            },
            primitive: {
                topology: options.topology || 'triangle-list',
                stripIndexFormat: options.stripIndexFormat,
                frontFace: 'ccw',
                cullMode: options.cullMode || 'back',
            },
            depthStencil: options.depthStencil,
            multisample: {
                count: options.sampleCount || 1,
            },
        });
    }
    
    // Create depth texture for rendering
    createDepthTexture() {
        return this.device.createTexture({
            size: [this.canvas.width, this.canvas.height],
            format: 'depth24plus',
            usage: GPUTextureUsage.RENDER_ATTACHMENT,
        });
    }
    
    // Helper to time GPU operations
    async timeGPUOperation(name, operation) {
        const start = performance.now();
        await operation();
        await this.device.queue.onSubmittedWorkDone();
        const elapsed = performance.now() - start;
        console.log(`[GPU] ${name} took ${elapsed.toFixed(2)}ms`);
        return elapsed;
    }
    
    // Get GPU memory info (if available)
    getMemoryInfo() {
        // Note: This API is not standardized yet
        if (this.adapter && this.adapter.requestAdapterInfo) {
            return this.adapter.requestAdapterInfo();
        }
        return null;
    }
}