// World State - Pure data structures for voxel world
// No classes, just typed arrays and buffers

import { createBuffer, writeBuffer } from './gpu-state.js';

// World configuration constants
export const WORLD_CONFIG = {
    size: 256,
    height: 128,
    chunkSize: 32,
    voxelSize: 4, // bytes per voxel (u32)
    metadataSize: 64, // bytes per chunk
    paletteSize: 256, // number of materials
};

// Calculate derived values
WORLD_CONFIG.chunksPerAxis = Math.floor(WORLD_CONFIG.size / WORLD_CONFIG.chunkSize);
WORLD_CONFIG.chunksPerAxisY = Math.floor(WORLD_CONFIG.height / WORLD_CONFIG.chunkSize);
WORLD_CONFIG.totalChunks = WORLD_CONFIG.chunksPerAxis * WORLD_CONFIG.chunksPerAxis * WORLD_CONFIG.chunksPerAxisY;
WORLD_CONFIG.totalVoxels = WORLD_CONFIG.size * WORLD_CONFIG.size * WORLD_CONFIG.height;
WORLD_CONFIG.voxelBufferSize = WORLD_CONFIG.totalVoxels * WORLD_CONFIG.voxelSize;
WORLD_CONFIG.metadataBufferSize = WORLD_CONFIG.totalChunks * WORLD_CONFIG.metadataSize;
WORLD_CONFIG.paletteBufferSize = WORLD_CONFIG.paletteSize * 4;

// World state - pure data
export const worldState = {
    // CPU-side typed arrays (for initialization only)
    palette: new Uint32Array(WORLD_CONFIG.paletteSize),
    
    // GPU buffers - the source of truth
    buffers: {
        voxel: null,
        metadata: null,
        palette: null,
        pageTable: null
    },
    
    // Bind group layouts
    bindGroupLayout: null,
    bindGroup: null,
    
    initialized: false
};

// Morton encoding - pure function for 32-bit
export function mortonEncode3D(x, y, z) {
    // Limit to 10 bits per component (0-1023)
    x = x & 0x3FF;
    y = y & 0x3FF;
    z = z & 0x3FF;
    
    // Spread bits
    x = (x | (x << 16)) & 0x030000FF;
    x = (x | (x << 8))  & 0x0300F00F;
    x = (x | (x << 4))  & 0x030C30C3;
    x = (x | (x << 2))  & 0x09249249;
    
    y = (y | (y << 16)) & 0x030000FF;
    y = (y | (y << 8))  & 0x0300F00F;
    y = (y | (y << 4))  & 0x030C30C3;
    y = (y | (y << 2))  & 0x09249249;
    
    z = (z | (z << 16)) & 0x030000FF;
    z = (z | (z << 8))  & 0x0300F00F;
    z = (z | (z << 4))  & 0x030C30C3;
    z = (z | (z << 2))  & 0x09249249;
    
    return x | (y << 1) | (z << 2);
}

// Pack RGBA color - pure function
export function packColor(r, g, b, a = 255) {
    return (a << 24) | (b << 16) | (g << 8) | r;
}

// Initialize palette data
export function initializePaletteData() {
    worldState.palette[0] = packColor(0, 0, 0, 0);         // Air
    worldState.palette[1] = packColor(139, 69, 19);        // Dirt
    worldState.palette[2] = packColor(34, 139, 34);        // Grass
    worldState.palette[3] = packColor(128, 128, 128);      // Stone
    worldState.palette[4] = packColor(0, 0, 255, 200);     // Water
    worldState.palette[5] = packColor(255, 215, 0);        // Gold
    
    // Fill rest with magenta
    for (let i = 6; i < WORLD_CONFIG.paletteSize; i++) {
        worldState.palette[i] = packColor(255, 0, 255);
    }
}

// Initialize world buffers - side effect function
export function initializeWorldBuffers() {
    console.log('[World] Creating GPU buffers...');
    
    // Create voxel buffer
    worldState.buffers.voxel = createBuffer(
        WORLD_CONFIG.voxelBufferSize,
        GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST | GPUBufferUsage.COPY_SRC,
        'WorldVoxels'
    );
    
    // Create metadata buffer
    worldState.buffers.metadata = createBuffer(
        WORLD_CONFIG.metadataBufferSize,
        GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
        'WorldMetadata'
    );
    
    // Create and initialize palette buffer
    initializePaletteData();
    worldState.buffers.palette = createBuffer(
        WORLD_CONFIG.paletteBufferSize,
        GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
        'WorldPalette',
        worldState.palette
    );
    
    // Create page table buffer
    worldState.buffers.pageTable = createBuffer(
        1024 * 1024, // 1MB
        GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
        'WorldPageTable'
    );
    
    worldState.initialized = true;
    console.log('[World] Buffers created:', {
        voxels: `${WORLD_CONFIG.voxelBufferSize / (1024*1024)}MB`,
        totalVoxels: WORLD_CONFIG.totalVoxels.toLocaleString()
    });
}

// Create world bind group layout - side effect function
export function createWorldBindGroupLayout(device) {
    worldState.bindGroupLayout = device.createBindGroupLayout({
        label: 'WorldBindGroupLayout',
        entries: [
            {
                binding: 0,
                visibility: GPUShaderStage.COMPUTE | GPUShaderStage.FRAGMENT,
                buffer: { type: 'storage' }
            },
            {
                binding: 1,
                visibility: GPUShaderStage.COMPUTE,
                buffer: { type: 'storage' }
            },
            {
                binding: 2,
                visibility: GPUShaderStage.COMPUTE | GPUShaderStage.FRAGMENT,
                buffer: { type: 'read-only-storage' }
            },
            {
                binding: 3,
                visibility: GPUShaderStage.COMPUTE,
                buffer: { type: 'storage' }
            }
        ]
    });
    
    worldState.bindGroup = device.createBindGroup({
        label: 'WorldBindGroup',
        layout: worldState.bindGroupLayout,
        entries: [
            { binding: 0, resource: { buffer: worldState.buffers.voxel } },
            { binding: 1, resource: { buffer: worldState.buffers.metadata } },
            { binding: 2, resource: { buffer: worldState.buffers.palette } },
            { binding: 3, resource: { buffer: worldState.buffers.pageTable } }
        ]
    });
}

// Debug function to read voxel from GPU
export async function debugReadVoxel(device, x, y, z) {
    const index = mortonEncode3D(x, y, z);
    const offset = index * 4;
    
    console.log(`[Debug] Reading voxel at (${x},${y},${z}), morton index: ${index}, offset: ${offset}`);
    
    // Bounds check
    if (offset >= WORLD_CONFIG.voxelBufferSize) {
        console.error(`[Debug] Offset ${offset} exceeds buffer size ${WORLD_CONFIG.voxelBufferSize}`);
        return 0; // Return air
    }
    
    const staging = device.createBuffer({
        size: 4,
        usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
    });
    
    const encoder = device.createCommandEncoder();
    encoder.copyBufferToBuffer(worldState.buffers.voxel, offset, staging, 0, 4);
    device.queue.submit([encoder.finish()]);
    
    await staging.mapAsync(GPUMapMode.READ);
    const data = new Uint32Array(staging.getMappedRange());
    const voxel = data[0];
    staging.unmap();
    staging.destroy();
    
    return voxel;
}