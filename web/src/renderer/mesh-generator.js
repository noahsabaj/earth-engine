// Mesh Generator - GPU-based voxel to mesh conversion
// Port of greedy meshing and GPU mesh generation

import { BUILTIN_SHADERS } from '../core/shader-loader.js';

export class MeshGenerator {
    constructor(device, worldBuffer) {
        this.device = device;
        this.worldBuffer = worldBuffer;
        
        // Buffers for generated mesh data
        this.vertexBuffer = null;
        this.indexBuffer = null;
        this.indirectBuffer = null;
        
        // Pipelines
        this.meshGenPipeline = null;
        this.bindGroup = null;
        
        // Stats
        this.totalVertices = 0;
        this.totalIndices = 0;
    }
    
    async init() {
        console.log('[Mesh] Initializing mesh generator...');
        
        // Create output buffers
        const maxVertices = 10 * 1024 * 1024; // 10M vertices
        const maxIndices = 15 * 1024 * 1024;  // 15M indices
        
        this.vertexBuffer = this.device.createBuffer({
            label: 'MeshVertexBuffer',
            size: maxVertices * 36, // 36 bytes per vertex (3 floats pos + 3 floats normal + 2 floats uv + 1 u32 color)
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_SRC,
        });
        
        this.indexBuffer = this.device.createBuffer({
            label: 'MeshIndexBuffer',
            size: maxIndices * 4, // u32 indices
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.INDEX | GPUBufferUsage.COPY_SRC,
        });
        
        // Indirect draw buffer for GPU-driven rendering
        this.indirectBuffer = this.device.createBuffer({
            label: 'IndirectDrawBuffer',
            size: 20, // DrawIndexedIndirect struct
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.INDIRECT | GPUBufferUsage.COPY_DST,
        });
        
        // Counter buffer to track generated vertices/indices
        this.counterBuffer = this.device.createBuffer({
            label: 'MeshCounterBuffer',
            size: 16, // vertex count + index count + padding
            usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
        });
        
        // Create mesh generation pipeline
        await this.createPipeline();
        
        console.log('[Mesh] Mesh generator initialized');
    }
    
    async createPipeline() {
        const shaderCode = this.createMeshGenShader();
        const shaderModule = this.device.createShaderModule({
            label: 'MeshGenShader',
            code: shaderCode,
        });
        
        this.meshGenPipeline = this.device.createComputePipeline({
            label: 'MeshGenPipeline',
            layout: 'auto',
            compute: {
                module: shaderModule,
                entryPoint: 'generate_mesh',
            }
        });
        
        // Create separate pipeline for finalizing indirect buffer
        this.finalizePipeline = this.device.createComputePipeline({
            label: 'FinalizePipeline',
            layout: 'auto',
            compute: {
                module: shaderModule,
                entryPoint: 'finalize_indirect',
            }
        });
        
        // Create bind group
        this.bindGroup = this.device.createBindGroup({
            label: 'MeshGenBindGroup',
            layout: this.meshGenPipeline.getBindGroupLayout(0),
            entries: [
                { binding: 0, resource: { buffer: this.worldBuffer.voxelBuffer } },
                { binding: 1, resource: { buffer: this.worldBuffer.paletteBuffer } },
                { binding: 2, resource: { buffer: this.vertexBuffer } },
                { binding: 3, resource: { buffer: this.indexBuffer } },
                { binding: 4, resource: { buffer: this.counterBuffer } },
                { binding: 5, resource: { buffer: this.indirectBuffer } },
            ]
        });
    }
    
    createMeshGenShader() {
        return `
            struct Vertex {
                position: vec3<f32>,
                normal: vec3<f32>,
                uv: vec2<f32>,
                color: u32,
            }
            
            struct DrawIndexedIndirect {
                index_count: u32,
                instance_count: u32,
                first_index: u32,
                base_vertex: i32,
                first_instance: u32,
            }
            
            struct Counters {
                vertex_count: atomic<u32>,
                index_count: atomic<u32>,
                _padding: vec2<u32>,
            }
            
            @group(0) @binding(0) var<storage, read> voxels: array<u32>;
            @group(0) @binding(1) var<storage, read> palette: array<u32>;
            @group(0) @binding(2) var<storage, read_write> vertices: array<Vertex>;
            @group(0) @binding(3) var<storage, read_write> indices: array<u32>;
            @group(0) @binding(4) var<storage, read_write> counters: Counters;
            @group(0) @binding(5) var<storage, read_write> indirect: DrawIndexedIndirect;
            
            ${BUILTIN_SHADERS.mortonEncode}
            
            const WORLD_SIZE = 256u;
            const WORLD_HEIGHT = 128u;
            const CHUNK_SIZE = 32u;
            
            fn get_voxel(x: u32, y: u32, z: u32) -> u32 {
                if (x >= WORLD_SIZE || y >= WORLD_HEIGHT || z >= WORLD_SIZE) {
                    return 0u;
                }
                let index = morton_encode_3d(x, y, z);
                return voxels[index];
            }
            
            fn is_face_visible(x: u32, y: u32, z: u32, nx: i32, ny: i32, nz: i32) -> bool {
                let voxel = get_voxel(x, y, z);
                if (voxel == 0u) { return false; } // Air
                
                let neighbor_x = u32(i32(x) + nx);
                let neighbor_y = u32(i32(y) + ny);
                let neighbor_z = u32(i32(z) + nz);
                
                let neighbor = get_voxel(neighbor_x, neighbor_y, neighbor_z);
                return neighbor == 0u || neighbor == 4u; // Air or water
            }
            
            fn add_face(pos: vec3<f32>, size: vec2<f32>, normal: vec3<f32>, color: u32, 
                       vertex_offset: u32, index_offset: u32) {
                // Generate vertices based on face normal
                var v0: vec3<f32>;
                var v1: vec3<f32>;
                var v2: vec3<f32>;
                var v3: vec3<f32>;
                
                if (abs(normal.y) > 0.5) {
                    // Top/bottom face
                    let y = pos.y + select(0.0, 1.0, normal.y > 0.0);
                    v0 = vec3<f32>(pos.x, y, pos.z);
                    v1 = vec3<f32>(pos.x + size.x, y, pos.z);
                    v2 = vec3<f32>(pos.x + size.x, y, pos.z + size.y);
                    v3 = vec3<f32>(pos.x, y, pos.z + size.y);
                } else if (abs(normal.x) > 0.5) {
                    // Left/right face
                    let x = pos.x + select(0.0, 1.0, normal.x > 0.0);
                    v0 = vec3<f32>(x, pos.y, pos.z);
                    v1 = vec3<f32>(x, pos.y, pos.z + size.y);
                    v2 = vec3<f32>(x, pos.y + size.x, pos.z + size.y);
                    v3 = vec3<f32>(x, pos.y + size.x, pos.z);
                } else {
                    // Front/back face
                    let z = pos.z + select(0.0, 1.0, normal.z > 0.0);
                    v0 = vec3<f32>(pos.x, pos.y, z);
                    v1 = vec3<f32>(pos.x + size.x, pos.y, z);
                    v2 = vec3<f32>(pos.x + size.x, pos.y + size.y, z);
                    v3 = vec3<f32>(pos.x, pos.y + size.y, z);
                }
                
                // Add vertices
                vertices[vertex_offset + 0u] = Vertex(v0, normal, vec2<f32>(0.0, 0.0), color);
                vertices[vertex_offset + 1u] = Vertex(v1, normal, vec2<f32>(1.0, 0.0), color);
                vertices[vertex_offset + 2u] = Vertex(v2, normal, vec2<f32>(1.0, 1.0), color);
                vertices[vertex_offset + 3u] = Vertex(v3, normal, vec2<f32>(0.0, 1.0), color);
                
                // Add indices (two triangles)
                let base = vertex_offset;
                indices[index_offset + 0u] = base + 0u;
                indices[index_offset + 1u] = base + 1u;
                indices[index_offset + 2u] = base + 2u;
                indices[index_offset + 3u] = base + 0u;
                indices[index_offset + 4u] = base + 2u;
                indices[index_offset + 5u] = base + 3u;
            }
            
            @compute @workgroup_size(8, 8, 8)
            fn generate_mesh(@builtin(global_invocation_id) id: vec3<u32>) {
                let chunk_pos = id;
                if (any(chunk_pos * CHUNK_SIZE >= vec3<u32>(WORLD_SIZE, WORLD_HEIGHT, WORLD_SIZE))) {
                    return;
                }
                
                let chunk_offset = chunk_pos * CHUNK_SIZE;
                
                // Process each voxel in chunk
                for (var y = 0u; y < CHUNK_SIZE; y++) {
                    for (var z = 0u; z < CHUNK_SIZE; z++) {
                        for (var x = 0u; x < CHUNK_SIZE; x++) {
                            let world_x = chunk_offset.x + x;
                            let world_y = chunk_offset.y + y;
                            let world_z = chunk_offset.z + z;
                            
                            let voxel = get_voxel(world_x, world_y, world_z);
                            if (voxel == 0u) { continue; }
                            
                            let color = palette[voxel];
                            let pos = vec3<f32>(f32(world_x), f32(world_y), f32(world_z));
                            
                            // Check each face
                            // Top (Y+)
                            if (is_face_visible(world_x, world_y, world_z, 0, 1, 0)) {
                                let vertex_idx = atomicAdd(&counters.vertex_count, 4u);
                                let index_idx = atomicAdd(&counters.index_count, 6u);
                                add_face(pos, vec2<f32>(1.0, 1.0), vec3<f32>(0.0, 1.0, 0.0), 
                                        color, vertex_idx, index_idx);
                            }
                            
                            // Bottom (Y-)
                            if (is_face_visible(world_x, world_y, world_z, 0, -1, 0)) {
                                let vertex_idx = atomicAdd(&counters.vertex_count, 4u);
                                let index_idx = atomicAdd(&counters.index_count, 6u);
                                add_face(pos, vec2<f32>(1.0, 1.0), vec3<f32>(0.0, -1.0, 0.0), 
                                        color, vertex_idx, index_idx);
                            }
                            
                            // Right (X+)
                            if (is_face_visible(world_x, world_y, world_z, 1, 0, 0)) {
                                let vertex_idx = atomicAdd(&counters.vertex_count, 4u);
                                let index_idx = atomicAdd(&counters.index_count, 6u);
                                add_face(pos, vec2<f32>(1.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), 
                                        color, vertex_idx, index_idx);
                            }
                            
                            // Left (X-)
                            if (is_face_visible(world_x, world_y, world_z, -1, 0, 0)) {
                                let vertex_idx = atomicAdd(&counters.vertex_count, 4u);
                                let index_idx = atomicAdd(&counters.index_count, 6u);
                                add_face(pos, vec2<f32>(1.0, 1.0), vec3<f32>(-1.0, 0.0, 0.0), 
                                        color, vertex_idx, index_idx);
                            }
                            
                            // Front (Z+)
                            if (is_face_visible(world_x, world_y, world_z, 0, 0, 1)) {
                                let vertex_idx = atomicAdd(&counters.vertex_count, 4u);
                                let index_idx = atomicAdd(&counters.index_count, 6u);
                                add_face(pos, vec2<f32>(1.0, 1.0), vec3<f32>(0.0, 0.0, 1.0), 
                                        color, vertex_idx, index_idx);
                            }
                            
                            // Back (Z-)
                            if (is_face_visible(world_x, world_y, world_z, 0, 0, -1)) {
                                let vertex_idx = atomicAdd(&counters.vertex_count, 4u);
                                let index_idx = atomicAdd(&counters.index_count, 6u);
                                add_face(pos, vec2<f32>(1.0, 1.0), vec3<f32>(0.0, 0.0, -1.0), 
                                        color, vertex_idx, index_idx);
                            }
                        }
                    }
                }
            }
            
            @compute @workgroup_size(1)
            fn finalize_indirect() {
                indirect.index_count = atomicLoad(&counters.index_count);
                indirect.instance_count = 1u;
                indirect.first_index = 0u;
                indirect.base_vertex = 0;
                indirect.first_instance = 0u;
            }
        `;
    }
    
    async generateMesh() {
        if (!this.meshGenPipeline) {
            await this.init();
        }
        
        console.log('[Mesh] Generating mesh...');
        console.log('[Mesh] World size:', this.worldBuffer.size, 'x', this.worldBuffer.height, 'x', this.worldBuffer.size);
        const startTime = performance.now();
        
        // Clear counters
        this.device.queue.writeBuffer(this.counterBuffer, 0, new Uint32Array([0, 0, 0, 0]));
        
        const encoder = this.device.createCommandEncoder();
        
        // Generate mesh
        {
            const pass = encoder.beginComputePass();
            pass.setPipeline(this.meshGenPipeline);
            pass.setBindGroup(0, this.bindGroup);
            
            // Dispatch one workgroup per chunk
            const chunksPerAxis = Math.ceil(this.worldBuffer.size / 32);
            const chunksY = Math.ceil(this.worldBuffer.height / 32);
            
            pass.dispatchWorkgroups(chunksPerAxis, chunksY, chunksPerAxis);
            pass.end();
        }
        
        // Finalize indirect buffer
        {
            const pass = encoder.beginComputePass();
            pass.setPipeline(this.finalizePipeline);
            pass.setBindGroup(0, this.bindGroup);
            pass.dispatchWorkgroups(1);
            pass.end();
        }
        
        this.device.queue.submit([encoder.finish()]);
        await this.device.queue.onSubmittedWorkDone();
        
        const elapsed = performance.now() - startTime;
        console.log(`[Mesh] Mesh generated in ${elapsed.toFixed(1)}ms`);
        
        // Read back stats (for debugging)
        await this.readStats();
    }
    
    async readStats() {
        const staging = this.device.createBuffer({
            size: 16,
            usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
        });
        
        const encoder = this.device.createCommandEncoder();
        encoder.copyBufferToBuffer(this.counterBuffer, 0, staging, 0, 16);
        this.device.queue.submit([encoder.finish()]);
        
        await staging.mapAsync(GPUMapMode.READ);
        const data = new Uint32Array(staging.getMappedRange());
        
        this.totalVertices = data[0];
        this.totalIndices = data[1];
        
        console.log(`[Mesh] Generated ${this.totalVertices.toLocaleString()} vertices, ${this.totalIndices.toLocaleString()} indices`);
        
        staging.unmap();
        staging.destroy();
    }
    
    getStats() {
        return {
            vertices: this.totalVertices,
            indices: this.totalIndices,
            triangles: Math.floor(this.totalIndices / 3),
        };
    }
}