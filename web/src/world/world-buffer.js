// WorldBuffer - The heart of our GPU-first architecture
// This is a direct port of src/world_gpu/world_buffer.rs

export class WorldBuffer {
    constructor(device, size = 256, height = 128) {
        this.device = device;
        this.size = size;
        this.height = height;
        this.chunksPerAxis = Math.floor(size / 32);
        this.totalChunks = this.chunksPerAxis * this.chunksPerAxis * Math.floor(height / 32);
        
        // Calculate buffer sizes - same as Rust
        this.voxelBufferSize = size * size * height * 4; // u32 per voxel
        this.metadataBufferSize = this.totalChunks * 64; // 64 bytes per chunk
        this.paletteBufferSize = 256 * 4; // 256 materials
    }
    
    async init() {
        // Create main voxel buffer - EXACT same as Rust
        this.voxelBuffer = this.device.createBuffer({
            label: "WorldBuffer.voxels",
            size: this.voxelBufferSize,
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST | GPUBufferUsage.COPY_SRC,
            mappedAtCreation: false,
        });
        
        // Chunk metadata buffer
        this.metadataBuffer = this.device.createBuffer({
            label: "WorldBuffer.metadata",
            size: this.metadataBufferSize,
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
            mappedAtCreation: false,
        });
        
        // Material palette buffer
        this.paletteBuffer = this.device.createBuffer({
            label: "WorldBuffer.palette",
            size: this.paletteBufferSize,
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
            mappedAtCreation: false,
        });
        
        // Page table for streaming (Sprint 23)
        this.pageTableBuffer = this.device.createBuffer({
            label: "WorldBuffer.pageTable",
            size: 1024 * 1024, // 1MB page table
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
            mappedAtCreation: false,
        });
        
        // Create bind group layout - matches Rust exactly
        this.bindGroupLayout = this.device.createBindGroupLayout({
            label: "WorldBuffer.bindGroupLayout",
            entries: [
                {
                    binding: 0,
                    visibility: GPUShaderStage.COMPUTE | GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
                    buffer: { type: "storage" }
                },
                {
                    binding: 1,
                    visibility: GPUShaderStage.COMPUTE,
                    buffer: { type: "storage" }
                },
                {
                    binding: 2,
                    visibility: GPUShaderStage.COMPUTE | GPUShaderStage.FRAGMENT,
                    buffer: { type: "read-only-storage" }
                },
                {
                    binding: 3,
                    visibility: GPUShaderStage.COMPUTE,
                    buffer: { type: "storage" }
                }
            ]
        });
        
        // Create bind group
        this.bindGroup = this.device.createBindGroup({
            label: "WorldBuffer.bindGroup",
            layout: this.bindGroupLayout,
            entries: [
                { binding: 0, resource: { buffer: this.voxelBuffer } },
                { binding: 1, resource: { buffer: this.metadataBuffer } },
                { binding: 2, resource: { buffer: this.paletteBuffer } },
                { binding: 3, resource: { buffer: this.pageTableBuffer } }
            ]
        });
        
        // Initialize material palette
        await this.initializePalette();
    }
    
    async initializePalette() {
        // Default materials - RGBA packed as ABGR for GPU
        const materials = new Uint32Array(256);
        
        // Helper to pack RGBA into GPU format (ABGR)
        const packColor = (r, g, b, a = 255) => {
            return (a << 24) | (b << 16) | (g << 8) | r;
        };
        
        materials[0] = packColor(0, 0, 0, 0);         // Air (transparent)
        materials[1] = packColor(139, 69, 19);        // Dirt (brown)
        materials[2] = packColor(34, 139, 34);        // Grass (green)
        materials[3] = packColor(128, 128, 128);      // Stone (gray)
        materials[4] = packColor(0, 0, 255, 200);     // Water (blue, semi-transparent)
        materials[5] = packColor(255, 215, 0);        // Ore (gold)
        
        // Fill rest with debug colors
        for (let i = 6; i < 256; i++) {
            materials[i] = packColor(255, 0, 255);    // Magenta for undefined
        }
        
        this.device.queue.writeBuffer(this.paletteBuffer, 0, materials);
        console.log('[WorldBuffer] Palette initialized with', materials.length, 'materials');
    }
    
    // Morton encoding for cache efficiency (Sprint 27)
    mortonEncode3D(x, y, z) {
        // Exact same algorithm as Rust
        x = (x | (x << 16)) & 0x030000FF0000FF;
        x = (x | (x << 8))  & 0x0300F00F00F00F;
        x = (x | (x << 4))  & 0x030C30C30C30C3;
        x = (x | (x << 2))  & 0x09249249249249;
        
        y = (y | (y << 16)) & 0x030000FF0000FF;
        y = (y | (y << 8))  & 0x0300F00F00F00F;
        y = (y | (y << 4))  & 0x030C30C30C30C3;
        y = (y | (y << 2))  & 0x09249249249249;
        
        z = (z | (z << 16)) & 0x030000FF0000FF;
        z = (z | (z << 8))  & 0x0300F00F00F00F;
        z = (z | (z << 4))  & 0x030C30C30C30C3;
        z = (z | (z << 2))  & 0x09249249249249;
        
        return x | (y << 1) | (z << 2);
    }
    
    // Get voxel at position (CPU-side, for debugging)
    async getVoxel(x, y, z) {
        const index = this.mortonEncode3D(x, y, z);
        const offset = index * 4;
        
        // Create staging buffer
        const stagingBuffer = this.device.createBuffer({
            size: 4,
            usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
        });
        
        // Copy from GPU
        const encoder = this.device.createCommandEncoder();
        encoder.copyBufferToBuffer(this.voxelBuffer, offset, stagingBuffer, 0, 4);
        this.device.queue.submit([encoder.finish()]);
        
        // Read back
        await stagingBuffer.mapAsync(GPUMapMode.READ);
        const data = new Uint32Array(stagingBuffer.getMappedRange());
        const voxel = data[0];
        stagingBuffer.unmap();
        stagingBuffer.destroy();
        
        return voxel;
    }
    
    // Set voxel (for editor, debugging)
    setVoxel(x, y, z, value) {
        const index = this.mortonEncode3D(x, y, z);
        const offset = index * 4;
        
        const data = new Uint32Array([value]);
        this.device.queue.writeBuffer(this.voxelBuffer, offset, data);
    }
    
    // Create compute pipeline bind group
    createComputeBindGroup(layout) {
        return this.device.createBindGroup({
            layout,
            entries: [
                { binding: 0, resource: { buffer: this.voxelBuffer } },
                { binding: 1, resource: { buffer: this.metadataBuffer } },
                { binding: 2, resource: { buffer: this.paletteBuffer } },
                { binding: 3, resource: { buffer: this.pageTableBuffer } }
            ]
        });
    }
    
    // Stats for debugging
    getStats() {
        return {
            worldSize: `${this.size}x${this.height}x${this.size}`,
            voxelCount: this.size * this.size * this.height,
            memoryUsage: (this.voxelBufferSize + this.metadataBufferSize) / (1024 * 1024),
            chunks: this.totalChunks,
        };
    }
}