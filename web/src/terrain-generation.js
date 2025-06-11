// Terrain Generation - Pure functions for GPU terrain generation
// No classes, just functions operating on data

import { createComputePipeline, createBuffer, writeBuffer, submitCommands, waitForGPU } from './gpu-state.js';
import { worldState, WORLD_CONFIG } from './world-state.js';
import { SHADER_SNIPPETS } from './shader-snippets.js';

// Terrain generation state
export const terrainState = {
    pipeline: null,
    paramsBuffer: null,
    bindGroup: null,
    initialized: false
};

// Terrain parameters structure
export function createTerrainParams(seed = 42, octaves = 6, frequency = 1.0, amplitude = 1.0) {
    const params = new ArrayBuffer(32);
    const view = new DataView(params);
    
    // World size (vec3<u32>)
    view.setUint32(0, WORLD_CONFIG.size, true);
    view.setUint32(4, WORLD_CONFIG.height, true);
    view.setUint32(8, WORLD_CONFIG.size, true);
    
    // Generation parameters
    view.setUint32(12, seed, true);
    view.setUint32(16, octaves, true);
    view.setFloat32(20, frequency, true);
    view.setFloat32(24, amplitude, true);
    
    return params;
}

// Create terrain generation shader code
export function createTerrainShader() {
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
        
        ${SHADER_SNIPPETS.mortonEncode}
        ${SHADER_SNIPPETS.noise3d}
        
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
        
        @compute @workgroup_size(8, 8, 8)
        fn generate_terrain(@builtin(global_invocation_id) id: vec3<u32>) {
            // Bounds check
            if (any(id >= params.world_size)) {
                return;
            }
            
            // DEBUG: Simple flat plane for testing
            var block_type = 0u; // Air
            
            // Create a small flat plane at y=50
            if (id.y == 50u && id.x < 20u && id.z < 20u) {
                block_type = 2u; // Grass
            }
            
            // Store using Morton encoding
            // Using linear indexing for debugging
            let index = morton_encode_3d(id.x, id.y, id.z);
            voxels[index] = block_type;
            
            // DEBUG: Force write at origin
            if (id.x == 0u && id.y == 50u && id.z == 0u) {
                voxels[index] = 5u; // Ensure gold block is written
            }
            
            // Update chunk metadata
            let chunk_pos = id / 32u;
            let chunk_index = chunk_pos.x + chunk_pos.y * (params.world_size.x / 32u) + 
                             chunk_pos.z * (params.world_size.x / 32u) * (params.world_size.y / 32u);
            metadata[chunk_index] = 1u;
        }
    `;
}

// Initialize terrain generation pipeline
export function initializeTerrainGeneration(device) {
    console.log('[Terrain] Creating generation pipeline...');
    
    // Create parameters buffer
    terrainState.paramsBuffer = createBuffer(
        32,
        GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
        'TerrainParams'
    );
    
    // Create compute pipeline
    const shaderCode = createTerrainShader();
    terrainState.pipeline = createComputePipeline(shaderCode, 'generate_terrain', 'TerrainGeneration');
    
    // Create bind group
    terrainState.bindGroup = device.createBindGroup({
        label: 'TerrainBindGroup',
        layout: terrainState.pipeline.getBindGroupLayout(0),
        entries: [
            { binding: 0, resource: { buffer: worldState.buffers.voxel } },
            { binding: 1, resource: { buffer: worldState.buffers.metadata } },
            { binding: 2, resource: { buffer: terrainState.paramsBuffer } }
        ]
    });
    
    terrainState.initialized = true;
    console.log('[Terrain] Pipeline created');
}

// Debug function to count voxels
export async function debugCountVoxels(device) {
    const shaderCode = `
        @group(0) @binding(0) var<storage, read> voxels: array<u32>;
        @group(0) @binding(1) var<storage, read_write> counter: atomic<u32>;
        
        @compute @workgroup_size(256)
        fn count_voxels(@builtin(global_invocation_id) id: vec3<u32>) {
            let idx = id.x;
            if (idx >= ${WORLD_CONFIG.totalVoxels}u) { return; }
            
            if (voxels[idx] != 0u) {
                atomicAdd(&counter, 1u);
            }
        }
    `;
    
    const pipeline = device.createComputePipeline({
        label: 'VoxelCounter',
        layout: 'auto',
        compute: {
            module: device.createShaderModule({ code: shaderCode }),
            entryPoint: 'count_voxels'
        }
    });
    
    const counterBuffer = device.createBuffer({
        size: 4,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
        mappedAtCreation: true
    });
    new Uint32Array(counterBuffer.getMappedRange())[0] = 0;
    counterBuffer.unmap();
    
    const bindGroup = device.createBindGroup({
        layout: pipeline.getBindGroupLayout(0),
        entries: [
            { binding: 0, resource: { buffer: worldState.buffers.voxel } },
            { binding: 1, resource: { buffer: counterBuffer } }
        ]
    });
    
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    pass.setPipeline(pipeline);
    pass.setBindGroup(0, bindGroup);
    pass.dispatchWorkgroups(Math.ceil(WORLD_CONFIG.totalVoxels / 256));
    pass.end();
    
    const staging = device.createBuffer({
        size: 4,
        usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ
    });
    encoder.copyBufferToBuffer(counterBuffer, 0, staging, 0, 4);
    device.queue.submit([encoder.finish()]);
    
    await staging.mapAsync(GPUMapMode.READ);
    const count = new Uint32Array(staging.getMappedRange())[0];
    staging.unmap();
    staging.destroy();
    counterBuffer.destroy();
    
    console.log(`[Terrain] Debug: Total non-zero voxels in buffer: ${count.toLocaleString()}`);
    return count;
}

// Generate terrain - main function
export async function generateTerrain(device, seed = 42) {
    if (!terrainState.initialized) {
        initializeTerrainGeneration(device);
    }
    
    console.log('[Terrain] Generating world...');
    const startTime = performance.now();
    
    // Update parameters
    const params = createTerrainParams(seed);
    writeBuffer(terrainState.paramsBuffer, 0, params);
    
    // Create command encoder
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginComputePass();
    
    pass.setPipeline(terrainState.pipeline);
    pass.setBindGroup(0, terrainState.bindGroup);
    
    // Dispatch workgroups
    const workgroupSize = 8;
    const dispatchX = Math.ceil(WORLD_CONFIG.size / workgroupSize);
    const dispatchY = Math.ceil(WORLD_CONFIG.height / workgroupSize);
    const dispatchZ = Math.ceil(WORLD_CONFIG.size / workgroupSize);
    
    pass.dispatchWorkgroups(dispatchX, dispatchY, dispatchZ);
    pass.end();
    
    // Submit and wait
    submitCommands([encoder.finish()]);
    await waitForGPU();
    
    const elapsed = performance.now() - startTime;
    const voxels = WORLD_CONFIG.totalVoxels;
    console.log(`[Terrain] Generated ${voxels.toLocaleString()} voxels in ${elapsed.toFixed(1)}ms`);
    console.log(`[Terrain] Performance: ${(voxels / (elapsed / 1000)).toLocaleString()} voxels/sec`);
}