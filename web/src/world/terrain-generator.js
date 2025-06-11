// Terrain Generator - GPU-based world generation
// Port of world_gpu/terrain_generator.rs

import { BUILTIN_SHADERS } from '../core/shader-loader.js';

export class TerrainGenerator {
    constructor(device, worldBuffer) {
        this.device = device;
        this.worldBuffer = worldBuffer;
        this.pipeline = null;
        this.bindGroup = null;
        this.paramsBuffer = null;
    }
    
    async init() {
        console.log('[Terrain] Initializing terrain generator...');
        
        // Create parameters buffer
        this.paramsBuffer = this.device.createBuffer({
            label: 'TerrainParams',
            size: 32, // vec3<u32> size + u32 seed + padding
            usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
        });
        
        // Create compute shader
        const shaderCode = this.createTerrainShader();
        const shaderModule = this.device.createShaderModule({
            label: 'TerrainGenerator',
            code: shaderCode,
        });
        
        // Create pipeline
        this.pipeline = this.device.createComputePipeline({
            label: 'TerrainPipeline',
            layout: 'auto',
            compute: {
                module: shaderModule,
                entryPoint: 'generate_terrain',
            }
        });
        
        // Create bind group
        this.bindGroup = this.device.createBindGroup({
            label: 'TerrainBindGroup',
            layout: this.pipeline.getBindGroupLayout(0),
            entries: [
                {
                    binding: 0,
                    resource: { buffer: this.worldBuffer.voxelBuffer }
                },
                {
                    binding: 1,
                    resource: { buffer: this.worldBuffer.metadataBuffer }
                },
                {
                    binding: 2,
                    resource: { buffer: this.paramsBuffer }
                }
            ]
        });
        
        console.log('[Terrain] Terrain generator initialized');
    }
    
    createTerrainShader() {
        return `
            struct TerrainParams {
                world_size: vec3<u32>,
                seed: u32,
                octaves: u32,
                frequency: f32,
                amplitude: f32,
                _padding: u32,
            }
            
            @group(0) @binding(0) var<storage, read_write> voxels: array<u32>;
            @group(0) @binding(1) var<storage, read_write> metadata: array<u32>;
            @group(0) @binding(2) var<uniform> params: TerrainParams;
            
            ${BUILTIN_SHADERS.mortonEncode}
            ${BUILTIN_SHADERS.noise3d}
            
            // Fractal Brownian Motion
            fn fbm(pos: vec3<f32>, octaves: u32, frequency: f32, amplitude: f32) -> f32 {
                var value = 0.0;
                var freq = frequency;
                var amp = amplitude;
                
                for (var i = 0u; i < octaves; i++) {
                    value += noise3d(pos * freq) * amp;
                    freq *= 2.0;
                    amp *= 0.5;
                }
                
                return value;
            }
            
            // Terrain density function
            fn terrain_density(world_pos: vec3<f32>) -> f32 {
                // Base terrain shape
                let height_factor = (world_pos.y / f32(params.world_size.y)) * 2.0 - 1.0;
                
                // Multi-octave noise
                let noise_value = fbm(world_pos * 0.01, params.octaves, params.frequency, params.amplitude);
                
                // Caves
                let cave_noise = fbm(world_pos * 0.05, 3u, 1.0, 0.5);
                let has_cave = cave_noise > 0.7;
                
                // Combine
                var density = noise_value - height_factor;
                if (has_cave && world_pos.y > 10.0) {
                    density -= 2.0;
                }
                
                return density;
            }
            
            @compute @workgroup_size(8, 8, 8)
            fn generate_terrain(@builtin(global_invocation_id) id: vec3<u32>) {
                // Bounds check
                if (any(id >= params.world_size)) {
                    return;
                }
                
                // TEMPORARY: Create a simple flat world for debugging
                var block_type = 0u; // Air
                
                // Create a flat plane at y=50
                if (id.y < 50u) {
                    block_type = 3u; // Stone
                } else if (id.y == 50u) {
                    block_type = 2u; // Grass
                }
                
                // Add some blocks above ground for visibility
                if (id.y == 51u && (id.x % 8u) == 0u && (id.z % 8u) == 0u) {
                    block_type = 1u; // Dirt pillars
                }
                
                // Store using Morton encoding for cache efficiency
                let index = morton_encode_3d(id.x, id.y, id.z);
                voxels[index] = block_type;
                
                // Update chunk metadata (simplified)
                let chunk_pos = id / 32u;
                let chunk_index = chunk_pos.x + chunk_pos.y * (params.world_size.x / 32u) + 
                                 chunk_pos.z * (params.world_size.x / 32u) * (params.world_size.y / 32u);
                metadata[chunk_index] = 1u; // Mark chunk as generated
            }
        `;
    }
    
    async generate(seed = 42) {
        if (!this.pipeline) {
            await this.init();
        }
        
        console.log('[Terrain] Generating world...');
        const startTime = performance.now();
        
        // Update parameters
        const params = new ArrayBuffer(32);
        const view = new DataView(params);
        
        // World size
        view.setUint32(0, this.worldBuffer.size, true);
        view.setUint32(4, this.worldBuffer.height, true);
        view.setUint32(8, this.worldBuffer.size, true);
        
        // Generation parameters
        view.setUint32(12, seed, true);
        view.setUint32(16, 6, true); // octaves
        view.setFloat32(20, 1.0, true); // frequency
        view.setFloat32(24, 1.0, true); // amplitude
        
        this.device.queue.writeBuffer(this.paramsBuffer, 0, params);
        
        // Run compute shader
        const encoder = this.device.createCommandEncoder();
        const pass = encoder.beginComputePass();
        
        pass.setPipeline(this.pipeline);
        pass.setBindGroup(0, this.bindGroup);
        
        // Dispatch enough workgroups to cover the world
        const workgroupSize = 8;
        const dispatchX = Math.ceil(this.worldBuffer.size / workgroupSize);
        const dispatchY = Math.ceil(this.worldBuffer.height / workgroupSize);
        const dispatchZ = Math.ceil(this.worldBuffer.size / workgroupSize);
        
        pass.dispatchWorkgroups(dispatchX, dispatchY, dispatchZ);
        pass.end();
        
        this.device.queue.submit([encoder.finish()]);
        
        // Wait for completion
        await this.device.queue.onSubmittedWorkDone();
        
        const elapsed = performance.now() - startTime;
        const voxels = this.worldBuffer.size * this.worldBuffer.height * this.worldBuffer.size;
        const voxelsPerSec = voxels / (elapsed / 1000);
        
        console.log(`[Terrain] Generated ${voxels.toLocaleString()} voxels in ${elapsed.toFixed(1)}ms`);
        console.log(`[Terrain] Performance: ${voxelsPerSec.toLocaleString()} voxels/sec`);
    }
    
    // Generate a specific chunk
    async generateChunk(chunkX, chunkY, chunkZ) {
        // This would dispatch compute for just one chunk
        // For now, we generate the whole world at once
        console.log(`[Terrain] Chunk generation at (${chunkX}, ${chunkY}, ${chunkZ})`);
    }
}