// Mesh Generation - Pure functions for GPU mesh generation
// No classes, just data and functions

import { createBuffer, createComputePipeline, writeBuffer, submitCommands, waitForGPU } from './gpu-state.js';
import { worldState, WORLD_CONFIG } from './world-state.js';
import { SHADER_SNIPPETS } from './shader-snippets.js';

// Mesh configuration
export const MESH_CONFIG = {
    maxVertices: 10 * 1024 * 1024,  // 10M vertices
    maxIndices: 15 * 1024 * 1024,   // 15M indices
    vertexSize: 36,                 // bytes per vertex
    indexSize: 4,                   // bytes per index (u32)
};

// Mesh state - pure data
export const meshState = {
    // GPU buffers
    buffers: {
        vertex: null,
        index: null,
        indirect: null,
        counter: null
    },
    
    // Pipelines
    pipelines: {
        generate: null,
        finalize: null
    },
    
    // Bind groups
    bindGroups: {
        generate: null,
        finalize: null
    },
    
    // Stats (read back from GPU)
    stats: {
        vertexCount: 0,
        indexCount: 0,
        triangleCount: 0
    },
    
    initialized: false
};

// Create mesh generation shader
export function createMeshGenerationShader() {
    return `
        ${SHADER_SNIPPETS.vertexStruct}
        ${SHADER_SNIPPETS.indirectDraw}
        ${SHADER_SNIPPETS.mortonEncode}
        
        struct Counters {
            vertex_count: atomic<u32>,
            index_count: atomic<u32>,
            voxel_count: atomic<u32>,
            visible_faces: atomic<u32>,
        }
        
        @group(0) @binding(0) var<storage, read> voxels: array<u32>;
        @group(0) @binding(1) var<storage, read> palette: array<u32>;
        @group(0) @binding(2) var<storage, read_write> vertices: array<Vertex>;
        @group(0) @binding(3) var<storage, read_write> indices: array<u32>;
        @group(0) @binding(4) var<storage, read_write> counters: Counters;
        @group(0) @binding(5) var<storage, read_write> indirect: DrawIndexedIndirect;
        
        const WORLD_SIZE = ${WORLD_CONFIG.size}u;
        const WORLD_HEIGHT = ${WORLD_CONFIG.height}u;
        const CHUNK_SIZE = ${WORLD_CONFIG.chunkSize}u;
        
        fn get_voxel(x: u32, y: u32, z: u32) -> u32 {
            if (x >= WORLD_SIZE || y >= WORLD_HEIGHT || z >= WORLD_SIZE) {
                return 0u;
            }
            let safe_x = min(x, 1023u);
            let safe_y = min(y, 1023u);
            let safe_z = min(z, 1023u);
            let index = morton_encode_3d(safe_x, safe_y, safe_z);
            return voxels[index];
        }
        
        fn is_face_visible(x: u32, y: u32, z: u32, nx: i32, ny: i32, nz: i32) -> bool {
            let voxel = get_voxel(x, y, z);
            if (voxel == 0u) { return false; }
            
            let nx_pos = i32(x) + nx;
            let ny_pos = i32(y) + ny;
            let nz_pos = i32(z) + nz;
            
            if (nx_pos < 0 || nx_pos >= i32(WORLD_SIZE) ||
                ny_pos < 0 || ny_pos >= i32(WORLD_HEIGHT) ||
                nz_pos < 0 || nz_pos >= i32(WORLD_SIZE)) {
                return true;
            }
            
            let neighbor = get_voxel(u32(nx_pos), u32(ny_pos), u32(nz_pos));
            return neighbor == 0u || neighbor == 4u;
        }
        
        fn add_face(pos: vec3<f32>, size: vec2<f32>, normal: vec3<f32>, color: u32, 
                   vertex_offset: u32, index_offset: u32) {
            var v0: vec3<f32>;
            var v1: vec3<f32>;
            var v2: vec3<f32>;
            var v3: vec3<f32>;
            
            if (abs(normal.y) > 0.5) {
                let y = pos.y + select(0.0, 1.0, normal.y > 0.0);
                v0 = vec3<f32>(pos.x, y, pos.z);
                v1 = vec3<f32>(pos.x + size.x, y, pos.z);
                v2 = vec3<f32>(pos.x + size.x, y, pos.z + size.y);
                v3 = vec3<f32>(pos.x, y, pos.z + size.y);
            } else if (abs(normal.x) > 0.5) {
                let x = pos.x + select(0.0, 1.0, normal.x > 0.0);
                v0 = vec3<f32>(x, pos.y, pos.z);
                v1 = vec3<f32>(x, pos.y, pos.z + size.x);
                v2 = vec3<f32>(x, pos.y + size.y, pos.z + size.x);
                v3 = vec3<f32>(x, pos.y + size.y, pos.z);
            } else {
                let z = pos.z + select(0.0, 1.0, normal.z > 0.0);
                v0 = vec3<f32>(pos.x, pos.y, z);
                v1 = vec3<f32>(pos.x + size.x, pos.y, z);
                v2 = vec3<f32>(pos.x + size.x, pos.y + size.y, z);
                v3 = vec3<f32>(pos.x, pos.y + size.y, z);
            }
            
            vertices[vertex_offset + 0u] = Vertex(v0, normal, vec2<f32>(0.0, 0.0), color);
            vertices[vertex_offset + 1u] = Vertex(v1, normal, vec2<f32>(1.0, 0.0), color);
            vertices[vertex_offset + 2u] = Vertex(v2, normal, vec2<f32>(1.0, 1.0), color);
            vertices[vertex_offset + 3u] = Vertex(v3, normal, vec2<f32>(0.0, 1.0), color);
            
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
            // Each thread processes one voxel directly
            let world_x = id.x;
            let world_y = id.y;
            let world_z = id.z;
            
            if (world_x >= WORLD_SIZE || world_y >= WORLD_HEIGHT || world_z >= WORLD_SIZE) {
                return;
            }
            
            let voxel = get_voxel(world_x, world_y, world_z);
            if (voxel == 0u) { return; }
            
            // Debug: Count non-zero voxels
            atomicAdd(&counters.voxel_count, 1u);
            
            let color = palette[voxel];
            let pos = vec3<f32>(f32(world_x), f32(world_y), f32(world_z));
            
            // Check each face
            if (is_face_visible(world_x, world_y, world_z, 0, 1, 0)) {
                atomicAdd(&counters.visible_faces, 1u);
                let vertex_idx = atomicAdd(&counters.vertex_count, 4u);
                let index_idx = atomicAdd(&counters.index_count, 6u);
                add_face(pos, vec2<f32>(1.0, 1.0), vec3<f32>(0.0, 1.0, 0.0), 
                        color, vertex_idx, index_idx);
            }
            
            if (is_face_visible(world_x, world_y, world_z, 0, -1, 0)) {
                atomicAdd(&counters.visible_faces, 1u);
                let vertex_idx = atomicAdd(&counters.vertex_count, 4u);
                let index_idx = atomicAdd(&counters.index_count, 6u);
                add_face(pos, vec2<f32>(1.0, 1.0), vec3<f32>(0.0, -1.0, 0.0), 
                        color, vertex_idx, index_idx);
            }
            
            if (is_face_visible(world_x, world_y, world_z, 1, 0, 0)) {
                atomicAdd(&counters.visible_faces, 1u);
                let vertex_idx = atomicAdd(&counters.vertex_count, 4u);
                let index_idx = atomicAdd(&counters.index_count, 6u);
                add_face(pos, vec2<f32>(1.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), 
                        color, vertex_idx, index_idx);
            }
            
            if (is_face_visible(world_x, world_y, world_z, -1, 0, 0)) {
                atomicAdd(&counters.visible_faces, 1u);
                let vertex_idx = atomicAdd(&counters.vertex_count, 4u);
                let index_idx = atomicAdd(&counters.index_count, 6u);
                add_face(pos, vec2<f32>(1.0, 1.0), vec3<f32>(-1.0, 0.0, 0.0), 
                        color, vertex_idx, index_idx);
            }
            
            if (is_face_visible(world_x, world_y, world_z, 0, 0, 1)) {
                atomicAdd(&counters.visible_faces, 1u);
                let vertex_idx = atomicAdd(&counters.vertex_count, 4u);
                let index_idx = atomicAdd(&counters.index_count, 6u);
                add_face(pos, vec2<f32>(1.0, 1.0), vec3<f32>(0.0, 0.0, 1.0), 
                        color, vertex_idx, index_idx);
            }
            
            if (is_face_visible(world_x, world_y, world_z, 0, 0, -1)) {
                atomicAdd(&counters.visible_faces, 1u);
                let vertex_idx = atomicAdd(&counters.vertex_count, 4u);
                let index_idx = atomicAdd(&counters.index_count, 6u);
                add_face(pos, vec2<f32>(1.0, 1.0), vec3<f32>(0.0, 0.0, -1.0), 
                        color, vertex_idx, index_idx);
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

// Initialize mesh generation
export function initializeMeshGeneration(device) {
    console.log('[Mesh] Creating buffers...');
    
    // Create buffers
    meshState.buffers.vertex = createBuffer(
        MESH_CONFIG.maxVertices * MESH_CONFIG.vertexSize,
        GPUBufferUsage.STORAGE | GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_SRC,
        'MeshVertices'
    );
    
    meshState.buffers.index = createBuffer(
        MESH_CONFIG.maxIndices * MESH_CONFIG.indexSize,
        GPUBufferUsage.STORAGE | GPUBufferUsage.INDEX | GPUBufferUsage.COPY_SRC,
        'MeshIndices'
    );
    
    meshState.buffers.indirect = createBuffer(
        20, // DrawIndexedIndirect size
        GPUBufferUsage.STORAGE | GPUBufferUsage.INDIRECT | GPUBufferUsage.COPY_DST,
        'MeshIndirect'
    );
    
    meshState.buffers.counter = createBuffer(
        16, // 2 atomics + padding
        GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC | GPUBufferUsage.COPY_DST,
        'MeshCounters'
    );
    
    // Create pipelines
    const shaderCode = createMeshGenerationShader();
    meshState.pipelines.generate = createComputePipeline(shaderCode, 'generate_mesh', 'MeshGeneration');
    meshState.pipelines.finalize = createComputePipeline(shaderCode, 'finalize_indirect', 'MeshFinalize');
    
    // Create bind groups - generate_mesh only uses bindings 0-4
    meshState.bindGroups.generate = device.createBindGroup({
        label: 'MeshGenerateBindGroup',
        layout: meshState.pipelines.generate.getBindGroupLayout(0),
        entries: [
            { binding: 0, resource: { buffer: worldState.buffers.voxel } },
            { binding: 1, resource: { buffer: worldState.buffers.palette } },
            { binding: 2, resource: { buffer: meshState.buffers.vertex } },
            { binding: 3, resource: { buffer: meshState.buffers.index } },
            { binding: 4, resource: { buffer: meshState.buffers.counter } }
        ]
    });
    
    // Finalize only uses bindings 4 and 5 (counters and indirect)
    meshState.bindGroups.finalize = device.createBindGroup({
        label: 'MeshFinalizeBindGroup',
        layout: meshState.pipelines.finalize.getBindGroupLayout(0),
        entries: [
            { binding: 4, resource: { buffer: meshState.buffers.counter } },
            { binding: 5, resource: { buffer: meshState.buffers.indirect } }
        ]
    });
    
    meshState.initialized = true;
    console.log('[Mesh] Initialization complete');
}

// Generate mesh from voxels
export async function generateMesh(device) {
    if (!meshState.initialized) {
        initializeMeshGeneration(device);
    }
    
    console.log('[Mesh] Generating mesh...');
    console.log('[Mesh] World config:', WORLD_CONFIG);
    const startTime = performance.now();
    
    // Clear counters
    writeBuffer(meshState.buffers.counter, 0, new Uint32Array([0, 0, 0, 0]));
    
    const encoder = device.createCommandEncoder();
    
    // Generate mesh
    {
        const pass = encoder.beginComputePass();
        pass.setPipeline(meshState.pipelines.generate);
        pass.setBindGroup(0, meshState.bindGroups.generate);
        
        // Each thread processes one voxel
        const workgroupSize = 8;
        const dispatchX = Math.ceil(WORLD_CONFIG.size / workgroupSize);
        const dispatchY = Math.ceil(WORLD_CONFIG.height / workgroupSize);
        const dispatchZ = Math.ceil(WORLD_CONFIG.size / workgroupSize);
        
        console.log(`[Mesh] Dispatching ${dispatchX}x${dispatchY}x${dispatchZ} workgroups`);
        console.log(`[Mesh] Each workgroup processes ${workgroupSize}³ voxels`);
        
        pass.dispatchWorkgroups(dispatchX, dispatchY, dispatchZ);
        pass.end();
    }
    
    // Finalize indirect buffer
    {
        const pass = encoder.beginComputePass();
        pass.setPipeline(meshState.pipelines.finalize);
        pass.setBindGroup(0, meshState.bindGroups.finalize);
        pass.dispatchWorkgroups(1);
        pass.end();
    }
    
    submitCommands([encoder.finish()]);
    await waitForGPU();
    
    const elapsed = performance.now() - startTime;
    console.log(`[Mesh] Generation complete in ${elapsed.toFixed(1)}ms`);
    
    // Read back stats
    await readMeshStats(device);
}

// Read mesh statistics from GPU
export async function readMeshStats(device) {
    const staging = device.createBuffer({
        size: 16,
        usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ,
    });
    
    const encoder = device.createCommandEncoder();
    encoder.copyBufferToBuffer(meshState.buffers.counter, 0, staging, 0, 16);
    device.queue.submit([encoder.finish()]);
    
    await staging.mapAsync(GPUMapMode.READ);
    const data = new Uint32Array(staging.getMappedRange());
    
    meshState.stats.vertexCount = data[0];
    meshState.stats.indexCount = data[1];
    meshState.stats.triangleCount = Math.floor(data[1] / 3);
    
    console.log(`[Mesh] Generated ${meshState.stats.vertexCount.toLocaleString()} vertices, ${meshState.stats.indexCount.toLocaleString()} indices`);
    console.log(`[Mesh] Debug: Found ${data[2].toLocaleString()} non-zero voxels, ${data[3].toLocaleString()} visible faces`);
    
    if (meshState.stats.vertexCount === 0) {
        console.warn('[Mesh] No vertices generated! Check terrain generation.');
        console.warn('[Mesh] Voxel count:', data[2], 'Visible faces:', data[3]);
    }
    
    staging.unmap();
    staging.destroy();
}