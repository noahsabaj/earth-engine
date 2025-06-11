// Engine - Main orchestration functions
// No classes, just functions coordinating all systems

console.log('[Engine] Loading engine.js module...');

import { gpuState, initializeGPU, resizeCanvas } from './gpu-state.js';
import { worldState, initializeWorldBuffers, createWorldBindGroupLayout, debugReadVoxel, WORLD_CONFIG } from './world-state.js';
import { generateTerrain } from './terrain-generation.js';
import { meshState, generateMesh } from './mesh-generation.js';
import { cameraState, initializeCamera, updateCamera } from './camera-state.js';
import { rendererState, initializeRenderer, renderFrame, resizeRenderer } from './renderer.js';

console.log('[Engine] All imports loaded');

// Engine state - pure data
export const engineState = {
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
async function generateWorld(seed = 42) {
    console.log('[Engine] Generating world...');
    const startTime = performance.now();
    
    // Generate terrain
    await generateTerrain(gpuState.device, seed);
    
    // Generate mesh from voxels
    await generateMesh(gpuState.device);
    
    const elapsed = performance.now() - startTime;
    console.log(`[Engine] World generation complete in ${elapsed.toFixed(1)}ms`);
    
    // Debug check
    const testVoxel = await debugReadVoxel(gpuState.device, 128, 50, 128);
    console.log('[Engine] Test voxel at (128,50,128):', testVoxel);
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

// Export engine API
export const engine = {
    initialize: initializeEngine,
    start: startEngine,
    stop: stopEngine,
    generateWorld,
    
    // Access to state for debugging
    state: {
        engine: engineState,
        gpu: gpuState,
        world: worldState,
        camera: cameraState,
        mesh: meshState,
        renderer: rendererState
    }
};

console.log('[Engine] Engine module loaded, exported:', engine);