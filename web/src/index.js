// Earth Engine JS - GPU-First Architecture
// This is the SAME engine as Rust, just different orchestration
// VERSION: 2.0 - Fixed initialization

console.log('===== INDEX.JS VERSION 2.0 LOADED =====');

import { EarthEngine } from './core/earth-engine.js';

// Create UI overlay
function createUI() {
    const ui = document.createElement('div');
    ui.id = 'engine-ui';
    ui.style.cssText = `
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
        user-select: none;
        z-index: 1000;
    `;
    
    const controls = document.createElement('div');
    controls.style.cssText = `
        position: fixed;
        top: 10px;
        right: 10px;
        color: white;
        font-family: monospace;
        font-size: 12px;
        background: rgba(0, 0, 0, 0.7);
        padding: 10px;
        border-radius: 5px;
        pointer-events: none;
        user-select: none;
        z-index: 1000;
    `;
    controls.innerHTML = `
        <div style="margin-bottom: 10px; font-size: 16px;">Earth Engine WebGPU v0.35.0</div>
        <div>Controls:</div>
        <div>WASD - Move</div>
        <div>Mouse - Look</div>
        <div>Space - Jump</div>
        <div>Shift - Sprint</div>
        <div>Click - Lock pointer</div>
    `;
    
    document.body.appendChild(ui);
    document.body.appendChild(controls);
    
    return ui;
}

// Update stats display
function updateStats(ui, engine) {
    const stats = engine.getStats();
    const cameraInfo = engine.getCameraInfo();
    const worldStats = engine.worldBuffer.getStats();
    
    ui.innerHTML = `
        <div style="margin-bottom: 10px;">Performance</div>
        <div>FPS: ${stats.fps}</div>
        <div>Frame: ${stats.frameTime.toFixed(2)}ms</div>
        <div>Draw calls: ${stats.drawCalls}</div>
        <div>Vertices: ${stats.vertices.toLocaleString()}</div>
        <div>Triangles: ${stats.triangles.toLocaleString()}</div>
        <div style="margin-top: 10px;">World</div>
        <div>Size: ${worldStats.worldSize}</div>
        <div>Voxels: ${worldStats.voxelCount.toLocaleString()}</div>
        <div>Memory: ${worldStats.memoryUsage.toFixed(1)}MB</div>
        <div>Chunks: ${worldStats.chunks}</div>
        <div style="margin-top: 10px;">Camera</div>
        <div>Pos: [${cameraInfo.position.join(', ')}]</div>
        <div>Yaw: ${cameraInfo.yaw}°</div>
        <div>Pitch: ${cameraInfo.pitch}°</div>
    `;
}

// Export everything needed for initialization
export { EarthEngine, createUI, updateStats };

// Helper function to initialize the engine
export async function initializeEngine(canvas) {
    console.log('[Engine] Starting initialization...');
    
    // Set canvas size
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    
    // Create UI
    console.log('[Engine] Creating UI...');
    const ui = createUI();
    
    // Create and initialize engine
    console.log('[Engine] Creating engine instance...');
    const engine = new EarthEngine(canvas);
    
    console.log('[Engine] Initializing engine...');
    await engine.init();
    
    console.log('[Engine] Starting render loop...');
    engine.start();
    
    // Update stats periodically
    setInterval(() => {
        updateStats(ui, engine);
    }, 100);
    
    // Handle window resize
    window.addEventListener('resize', () => {
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;
    });
    
    // Expose to window for debugging
    window.engine = engine;
    window.EarthEngine = EarthEngine;
    
    console.log('[Engine] Earth Engine started successfully!');
    console.log('[Engine] Use window.engine to access the engine instance');
    
    return engine;
}