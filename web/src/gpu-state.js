// GPU State - Pure data structures for GPU management
// No classes, just data and functions

// GPU state object - holds all GPU-related data
export const gpuState = {
    adapter: null,
    device: null,
    context: null,
    canvas: null,
    presentationFormat: 'bgra8unorm',
    initialized: false
};

// Initialize GPU - pure function that modifies state
export async function initializeGPU(canvas) {
    console.log('[GPU] Initializing WebGPU...');
    
    if (!navigator.gpu) {
        throw new Error('WebGPU not supported in this browser');
    }
    
    // Request adapter
    gpuState.adapter = await navigator.gpu.requestAdapter({
        powerPreference: 'high-performance',
    });
    
    if (!gpuState.adapter) {
        throw new Error('No GPU adapter found');
    }
    
    // Request device
    gpuState.device = await gpuState.adapter.requestDevice({
        requiredFeatures: [],
        requiredLimits: {
            maxBufferSize: 2147483648, // 2GB
            maxStorageBufferBindingSize: 2147483648,
            maxComputeWorkgroupStorageSize: 32768,
            maxComputeInvocationsPerWorkgroup: 512,
        }
    });
    
    // Configure canvas
    gpuState.canvas = canvas;
    gpuState.context = canvas.getContext('webgpu');
    
    if (!gpuState.context) {
        throw new Error('Failed to get WebGPU context');
    }
    
    // Configure swap chain
    gpuState.context.configure({
        device: gpuState.device,
        format: gpuState.presentationFormat,
        alphaMode: 'premultiplied',
    });
    
    gpuState.initialized = true;
    
    console.log('[GPU] WebGPU initialized successfully');
    return gpuState;
}

// Resize canvas - pure function
export function resizeCanvas(width, height) {
    if (!gpuState.initialized) return;
    
    gpuState.canvas.width = width;
    gpuState.canvas.height = height;
    
    // Reconfigure context
    gpuState.context.configure({
        device: gpuState.device,
        format: gpuState.presentationFormat,
        alphaMode: 'premultiplied',
    });
}

// Create shader module - pure function
export function createShaderModule(code, label = 'Shader') {
    return gpuState.device.createShaderModule({
        label,
        code
    });
}

// Create compute pipeline - pure function
export function createComputePipeline(shaderCode, entryPoint = 'main', label = 'ComputePipeline') {
    const shaderModule = createShaderModule(shaderCode, `${label}_Shader`);
    
    return gpuState.device.createComputePipeline({
        label,
        layout: 'auto',
        compute: {
            module: shaderModule,
            entryPoint
        }
    });
}

// Create render pipeline - pure function
export function createRenderPipeline(vertexCode, fragmentCode, options = {}) {
    const vertexModule = createShaderModule(vertexCode, 'VertexShader');
    const fragmentModule = createShaderModule(fragmentCode, 'FragmentShader');
    
    const pipeline = gpuState.device.createRenderPipeline({
        label: options.label || 'RenderPipeline',
        layout: options.layout || 'auto',
        vertex: {
            module: vertexModule,
            entryPoint: options.vertexEntry || 'vs_main',
            buffers: options.vertexBuffers || []
        },
        fragment: {
            module: fragmentModule,
            entryPoint: options.fragmentEntry || 'fs_main',
            targets: [{
                format: gpuState.presentationFormat,
                blend: options.blend
            }]
        },
        primitive: {
            topology: options.topology || 'triangle-list',
            cullMode: options.cullMode || 'back',
            frontFace: options.frontFace || 'ccw'
        },
        depthStencil: options.depthStencil
    });
    
    return pipeline;
}

// Create buffer - pure function
export function createBuffer(size, usage, label = 'Buffer', data = null) {
    const buffer = gpuState.device.createBuffer({
        label,
        size,
        usage,
        mappedAtCreation: data !== null
    });
    
    if (data !== null) {
        const mappedRange = buffer.getMappedRange();
        new Uint8Array(mappedRange).set(new Uint8Array(data.buffer));
        buffer.unmap();
    }
    
    return buffer;
}

// Write to buffer - pure function
export function writeBuffer(buffer, offset, data) {
    gpuState.device.queue.writeBuffer(buffer, offset, data);
}

// Create texture - pure function
export function createTexture(width, height, format, usage, label = 'Texture') {
    return gpuState.device.createTexture({
        label,
        size: [width, height, 1],
        format,
        usage
    });
}

// Submit commands - pure function
export function submitCommands(commandBuffers) {
    gpuState.device.queue.submit(commandBuffers);
}

// Wait for GPU - pure function
export async function waitForGPU() {
    await gpuState.device.queue.onSubmittedWorkDone();
}