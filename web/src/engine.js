// Engine - Main orchestration functions
// No classes, just functions coordinating all systems

console.log('[Engine] Loading engine.js module...');

import { gpuState, initializeGPU, resizeCanvas } from './gpu-state.js';
import { worldState, initializeWorldBuffers, createWorldBindGroupLayout, WORLD_CONFIG } from './world-state.js';
import { generateTerrain } from './terrain-generation.js';
import { meshState, generateMesh } from './mesh-generation.js';
import { cameraState, initializeCamera, updateCamera } from './camera-state.js';
import { rendererState, initializeRenderer, renderFrame, resizeRenderer } from './renderer.js';

console.log('[Engine] All imports loaded');

// Engine state - pure data
const engineState = {
    canvas: null,
    isRunning: false,
    lastFrameTime: 0,
    deltaTime: 0,
    frameCount: 0,
    
    // Stats overlay element
    statsElement: null,
    
    initialized: false
};

// Initialize the engine
async function initializeEngine(canvas) {
    console.log('[Engine] Starting initialization...');
    engineState.canvas = canvas;
    
    try {
        // Initialize GPU
        await initializeGPU(canvas);
        
        // Initialize world buffers
        initializeWorldBuffers();
        createWorldBindGroupLayout(gpuState.device);
        
        // Initialize camera
        initializeCamera(canvas);
        
        // Initialize renderer
        initializeRenderer();
        
        // Generate initial world
        await generateWorld();
        
        // Setup event handlers
        setupEventHandlers();
        
        // Create stats overlay
        createStatsOverlay();
        
        engineState.initialized = true;
        console.log('[Engine] Initialization complete!');
        
        // Log world stats
        console.log('[Engine] World stats:', {
            size: `${WORLD_CONFIG.size}x${WORLD_CONFIG.height}x${WORLD_CONFIG.size}`,
            voxels: WORLD_CONFIG.totalVoxels.toLocaleString(),
            memory: `${(worldState.buffers.voxel.size / (1024*1024)).toFixed(1)}MB`
        });
        
    } catch (error) {
        console.error('[Engine] Failed to initialize:', error);
        throw error;
    }
}

// Generate world (terrain + mesh)
// Debug: Read first few vertices
async function debugReadVertices() {
    const count = 5;
    const staging = gpuState.device.createBuffer({
        size: 36 * count, // 5 vertices
        usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ
    });
    
    const encoder = gpuState.device.createCommandEncoder();
    encoder.copyBufferToBuffer(meshState.buffers.vertex, 0, staging, 0, 36 * count);
    gpuState.device.queue.submit([encoder.finish()]);
    
    await staging.mapAsync(GPUMapMode.READ);
    const floatData = new Float32Array(staging.getMappedRange());
    
    for (let i = 0; i < count; i++) {
        const offset = i * 9; // 9 floats per vertex (36 bytes / 4)
        console.log(`[Debug] Vertex ${i}:`, {
            position: [floatData[offset], floatData[offset+1], floatData[offset+2]],
            normal: [floatData[offset+3], floatData[offset+4], floatData[offset+5]],
            uv: [floatData[offset+6], floatData[offset+7]],
            colorRaw: floatData[offset+8]
        });
    }
    
    staging.unmap();
    staging.destroy();
}

// Debug: Create simple test world
async function debugCreateTestWorld() {
    console.log('[Debug] Creating test world...');
    
    // Clear voxel buffer first
    const encoder = gpuState.device.createCommandEncoder();
    encoder.clearBuffer(worldState.buffers.voxel, 0);
    gpuState.device.queue.submit([encoder.finish()]);
    
    const shaderCode = `
        @group(0) @binding(0) var<storage, read_write> voxels: array<u32>;
        
        // Linear indexing for debugging
        fn get_index(x: u32, y: u32, z: u32) -> u32 {
            return y * 256u * 256u + z * 256u + x;
        }
        
        @compute @workgroup_size(1)
        fn test_world() {
            // Create a large floor at y=50
            for (var z = 0u; z < 100u; z++) {
                for (var x = 0u; x < 100u; x++) {
                    let idx = get_index(x, 50u, z);
                    voxels[idx] = 5u; // Gold blocks
                }
            }
            
            // Create a pillar at center
            for (var y = 51u; y < 60u; y++) {
                let idx = get_index(50u, y, 50u);
                voxels[idx] = 3u; // Stone blocks
            }
        }
    `;
    
    const pipeline = gpuState.device.createComputePipeline({
        label: 'TestWorld',
        layout: 'auto',
        compute: {
            module: gpuState.device.createShaderModule({ code: shaderCode }),
            entryPoint: 'test_world'
        }
    });
    
    const bindGroup = gpuState.device.createBindGroup({
        layout: pipeline.getBindGroupLayout(0),
        entries: [
            { binding: 0, resource: { buffer: worldState.buffers.voxel } }
        ]
    });
    
    const encoder2 = gpuState.device.createCommandEncoder();
    const pass = encoder2.beginComputePass();
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.dispatchWorkgroups(1);
    pass.end();
    gpuState.device.queue.submit([encoder2.finish()]);
    
    // Wait for GPU
    await gpuState.device.queue.onSubmittedWorkDone();
}

// Generate world (terrain + mesh)
async function generateWorld(seed = 42) {
    console.log('[Engine] Generating world...');
    const startTime = performance.now();
    
    // Generate terrain
    await generateTerrain(gpuState.device, seed);
    
    // Generate mesh from voxels
    await generateMesh(gpuState.device);
    
    const elapsed = performance.now() - startTime;
    console.log(`[Engine] World generation complete in ${elapsed.toFixed(1)}ms`);
    
}

// Start the engine
function startEngine() {
    if (engineState.isRunning || !engineState.initialized) return;
    
    console.log('[Engine] Starting render loop...');
    engineState.isRunning = true;
    engineState.lastFrameTime = performance.now();
    requestAnimationFrame(tick);
}

// Stop the engine
function stopEngine() {
    console.log('[Engine] Stopping render loop...');
    engineState.isRunning = false;
}

// Main update loop
function tick(currentTime) {
    if (!engineState.isRunning) return;
    
    // Calculate delta time
    engineState.deltaTime = (currentTime - engineState.lastFrameTime) / 1000;
    engineState.lastFrameTime = currentTime;
    
    // Update systems
    update(engineState.deltaTime);
    
    // Render
    render();
    
    // Update stats
    updateStats();
    
    // Continue loop
    requestAnimationFrame(tick);
}

// Update all systems
function update(deltaTime) {
    // Update camera from input
    updateCamera(deltaTime);
    
    // Future: Update physics, entities, streaming, etc.
}

// Render frame
function render() {
    renderFrame();
}

// Update stats display
function updateStats() {
    engineState.frameCount++;
    
    // Update every 10 frames
    if (engineState.frameCount % 10 === 0 && engineState.statsElement) {
        const stats = {
            fps: rendererState.stats.fps,
            vertices: meshState.stats.vertexCount.toLocaleString(),
            triangles: meshState.stats.triangleCount.toLocaleString(),
            position: `${Math.floor(cameraState.position[0])}, ${Math.floor(cameraState.position[1])}, ${Math.floor(cameraState.position[2])}`,
            yaw: (cameraState.rotation[0] * 180 / Math.PI).toFixed(1),
            pitch: (cameraState.rotation[1] * 180 / Math.PI).toFixed(1)
        };
        
        engineState.statsElement.innerHTML = `
            <div>FPS: ${stats.fps}</div>
            <div>Vertices: ${stats.vertices}</div>
            <div>Triangles: ${stats.triangles}</div>
            <div>Position: ${stats.position}</div>
            <div>Rotation: ${stats.yaw}°, ${stats.pitch}°</div>
            <div>GPU: ${(worldState.buffers.voxel.size / (1024*1024)).toFixed(1)}MB</div>
        `;
    }
}

// Setup event handlers
function setupEventHandlers() {
    // Handle window resize
    window.addEventListener('resize', () => {
        engineState.canvas.width = window.innerWidth;
        engineState.canvas.height = window.innerHeight;
        cameraState.aspect = engineState.canvas.width / engineState.canvas.height;
        resizeCanvas(window.innerWidth, window.innerHeight);
        resizeRenderer();
    });
    
    // Handle visibility change
    document.addEventListener('visibilitychange', () => {
        if (document.hidden) {
            engineState.isRunning = false;
        } else if (engineState.initialized) {
            startEngine();
        }
    });
}

// Create stats overlay
function createStatsOverlay() {
    engineState.statsElement = document.createElement('div');
    engineState.statsElement.style.cssText = `
        position: fixed;
        top: 10px;
        left: 10px;
        color: white;
        font-family: monospace;
        font-size: 14px;
        background: rgba(0, 0, 0, 0.7);
        padding: 10px;
        border-radius: 5px;
        pointer-events: none;
        z-index: 1000;
    `;
    document.body.appendChild(engineState.statsElement);
    
    // Controls help
    const controls = document.createElement('div');
    controls.style.cssText = `
        position: fixed;
        top: 10px;
        right: 10px;
        color: white;
        font-family: monospace;
        font-size: 14px;
        background: rgba(0, 0, 0, 0.7);
        padding: 10px;
        border-radius: 5px;
        pointer-events: none;
        z-index: 1000;
    `;
    controls.innerHTML = `
        <div><b>Controls:</b></div>
        <div>WASD - Move</div>
        <div>Mouse - Look</div>
        <div>Space - Up</div>
        <div>Shift - Down</div>
        <div>Click - Lock pointer</div>
    `;
    document.body.appendChild(controls);
}

// Export pure functions
export { initializeEngine, startEngine, stopEngine, generateWorld };

// Export states for debugging
export { engineState, gpuState, worldState, cameraState, meshState, rendererState };

console.log('[Engine] Engine module loaded');