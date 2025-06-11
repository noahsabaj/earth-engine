// EarthEngine - Main engine coordinating all GPU systems
// JavaScript implementation parallel to Rust's architecture

import { GPUContext } from './gpu-context.js';
import { WorldBuffer } from '../world/world-buffer.js';
import { TerrainGenerator } from '../world/terrain-generator.js';
import { MeshGenerator } from '../renderer/mesh-generator.js';
import { GPURenderer } from '../renderer/gpu-renderer.js';
import { CameraData, CameraController } from '../renderer/camera.js';

export class EarthEngine {
    constructor(canvas) {
        this.canvas = canvas;
        
        // Core systems
        this.gpu = null;
        this.worldBuffer = null;
        this.terrainGenerator = null;
        this.meshGenerator = null;
        this.renderer = null;
        this.camera = null;
        this.cameraController = null;
        
        // Engine state
        this.isRunning = false;
        this.lastFrameTime = 0;
        this.frameCount = 0;
        this.deltaTime = 0;
        
        // Performance tracking
        this.stats = {
            fps: 0,
            frameTime: 0,
            drawCalls: 0,
            vertices: 0,
            triangles: 0,
            gpuMemory: 0,
            cpuTime: 0,
            gpuTime: 0,
        };
    }
    
    async init() {
        console.log('[Engine] Initializing Earth Engine...');
        
        try {
            // Initialize GPU context
            this.gpu = new GPUContext(this.canvas);
            await this.gpu.init();
            
            // Create world buffer - the heart of our architecture
            this.worldBuffer = new WorldBuffer(this.gpu.device, 256, 128);
            await this.worldBuffer.init();
            
            // Initialize terrain generator
            this.terrainGenerator = new TerrainGenerator(this.gpu.device, this.worldBuffer);
            await this.terrainGenerator.init();
            
            // Initialize mesh generator
            this.meshGenerator = new MeshGenerator(this.gpu.device, this.worldBuffer);
            await this.meshGenerator.init();
            
            // Initialize renderer
            this.renderer = new GPURenderer(this.gpu, this.worldBuffer, this.meshGenerator);
            await this.renderer.init();
            
            // Setup camera
            this.camera = new CameraData({
                position: [128, 55, 128],  // Just above the ground at y=50
                yaw: Math.PI / 4,
                pitch: -0.3,
                fov: Math.PI / 3,
                aspect: this.canvas.width / this.canvas.height,
                near: 0.1,
                far: 1000
            });
            
            this.cameraController = new CameraController(this.camera);
            
            // Generate initial world
            await this.generateWorld();
            
            // Handle resize
            this.setupEventHandlers();
            
            console.log('[Engine] Earth Engine initialized successfully!');
            console.log('[Engine] World stats:', this.worldBuffer.getStats());
            
        } catch (error) {
            console.error('[Engine] Failed to initialize:', error);
            throw error;
        }
    }
    
    async generateWorld(seed = 42) {
        console.log('[Engine] Generating world...');
        const startTime = performance.now();
        
        // Generate terrain on GPU
        await this.terrainGenerator.generate(seed);
        
        // Generate mesh from voxels
        await this.meshGenerator.generateMesh();
        
        const elapsed = performance.now() - startTime;
        console.log(`[Engine] World generation complete in ${elapsed.toFixed(1)}ms`);
    }
    
    setupEventHandlers() {
        // Handle window resize
        window.addEventListener('resize', () => {
            this.canvas.width = window.innerWidth;
            this.canvas.height = window.innerHeight;
            this.camera.aspect = this.canvas.width / this.canvas.height;
            this.renderer.resize();
        });
        
        // Handle visibility change
        document.addEventListener('visibilitychange', () => {
            if (document.hidden) {
                this.pause();
            } else {
                this.resume();
            }
        });
    }
    
    start() {
        if (this.isRunning) return;
        
        console.log('[Engine] Starting render loop...');
        this.isRunning = true;
        this.lastFrameTime = performance.now();
        this.animate();
    }
    
    stop() {
        console.log('[Engine] Stopping render loop...');
        this.isRunning = false;
    }
    
    pause() {
        this.isRunning = false;
    }
    
    resume() {
        if (!this.isRunning) {
            this.isRunning = true;
            this.lastFrameTime = performance.now();
            this.animate();
        }
    }
    
    animate() {
        if (!this.isRunning) return;
        
        const now = performance.now();
        this.deltaTime = (now - this.lastFrameTime) / 1000;
        this.lastFrameTime = now;
        
        // Update systems
        this.update(this.deltaTime);
        
        // Render frame
        this.render();
        
        // Update stats
        this.updateStats();
        
        // Continue loop
        requestAnimationFrame(() => this.animate());
    }
    
    update(deltaTime) {
        // Update camera from input
        this.cameraController.update(deltaTime);
        
        // Update other systems here
        // - Physics
        // - Entity updates
        // - World streaming
        // - etc.
    }
    
    render() {
        // All rendering happens on GPU!
        this.renderer.render(this.camera);
    }
    
    updateStats() {
        this.frameCount++;
        
        // Update FPS every second
        if (this.frameCount % 60 === 0) {
            const renderStats = this.renderer.getStats();
            const meshStats = this.meshGenerator.getStats();
            
            this.stats = {
                fps: renderStats.fps,
                frameTime: this.deltaTime * 1000,
                drawCalls: renderStats.drawCalls,
                vertices: meshStats.vertices,
                triangles: meshStats.triangles,
                gpuMemory: this.worldBuffer.voxelBufferSize / (1024 * 1024),
                cpuTime: 0, // TODO: Measure
                gpuTime: 0, // TODO: GPU timing queries
            };
        }
    }
    
    getStats() {
        return this.stats;
    }
    
    // World editing API
    async setVoxel(x, y, z, blockType) {
        this.worldBuffer.setVoxel(x, y, z, blockType);
        
        // Regenerate affected chunk mesh
        await this.meshGenerator.generateMesh();
    }
    
    async getVoxel(x, y, z) {
        return await this.worldBuffer.getVoxel(x, y, z);
    }
    
    // Debug utilities
    takeScreenshot() {
        const dataURL = this.canvas.toDataURL('image/png');
        const link = document.createElement('a');
        link.download = `earth-engine-${Date.now()}.png`;
        link.href = dataURL;
        link.click();
    }
    
    getCameraInfo() {
        return this.cameraController.getDebugInfo();
    }
    
    // Resource cleanup
    destroy() {
        console.log('[Engine] Cleaning up resources...');
        
        this.stop();
        
        // Cleanup GPU resources
        if (this.worldBuffer) {
            this.worldBuffer.voxelBuffer?.destroy();
            this.worldBuffer.metadataBuffer?.destroy();
            this.worldBuffer.paletteBuffer?.destroy();
        }
        
        if (this.meshGenerator) {
            this.meshGenerator.vertexBuffer?.destroy();
            this.meshGenerator.indexBuffer?.destroy();
            this.meshGenerator.indirectBuffer?.destroy();
        }
        
        if (this.renderer) {
            this.renderer.depthTexture?.destroy();
            this.renderer.cameraBuffer?.destroy();
        }
        
        console.log('[Engine] Cleanup complete');
    }
}