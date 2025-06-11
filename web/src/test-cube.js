// Test cube generator - bypasses all the complex mesh generation
export class TestCubeGenerator {
    constructor(device) {
        this.device = device;
        this.vertexBuffer = null;
        this.indexBuffer = null;
        this.totalVertices = 0;
        this.totalIndices = 0;
    }
    
    async init() {
        console.log('[TestCube] Creating simple cube mesh...');
        
        // Create a single cube at origin
        const vertices = new Float32Array([
            // Position (3), Normal (3), UV (2), Color (1) = 9 floats per vertex
            // Front face
            -1, -1,  1,  0,  0,  1,  0, 0,  0xFF00FF00, // 0
             1, -1,  1,  0,  0,  1,  1, 0,  0xFF00FF00, // 1
             1,  1,  1,  0,  0,  1,  1, 1,  0xFF00FF00, // 2
            -1,  1,  1,  0,  0,  1,  0, 1,  0xFF00FF00, // 3
            
            // Back face
            -1, -1, -1,  0,  0, -1,  0, 0,  0xFF0000FF, // 4
            -1,  1, -1,  0,  0, -1,  0, 1,  0xFF0000FF, // 5
             1,  1, -1,  0,  0, -1,  1, 1,  0xFF0000FF, // 6
             1, -1, -1,  0,  0, -1,  1, 0,  0xFF0000FF, // 7
            
            // Top face
            -1,  1, -1,  0,  1,  0,  0, 0,  0xFFFF0000, // 8
            -1,  1,  1,  0,  1,  0,  0, 1,  0xFFFF0000, // 9
             1,  1,  1,  0,  1,  0,  1, 1,  0xFFFF0000, // 10
             1,  1, -1,  0,  1,  0,  1, 0,  0xFFFF0000, // 11
        ]);
        
        const indices = new Uint32Array([
            // Front
            0, 1, 2,
            0, 2, 3,
            // Back
            4, 5, 6,
            4, 6, 7,
            // Top
            8, 9, 10,
            8, 10, 11,
        ]);
        
        // Create buffers
        this.vertexBuffer = this.device.createBuffer({
            label: 'TestCube vertices',
            size: vertices.byteLength,
            usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
        });
        
        this.indexBuffer = this.device.createBuffer({
            label: 'TestCube indices',
            size: indices.byteLength,
            usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST,
        });
        
        // Upload data
        this.device.queue.writeBuffer(this.vertexBuffer, 0, vertices);
        this.device.queue.writeBuffer(this.indexBuffer, 0, indices);
        
        this.totalVertices = 12;
        this.totalIndices = 18;
        
        // Create indirect buffer for compatibility
        this.indirectBuffer = this.device.createBuffer({
            label: 'TestCube indirect',
            size: 20,
            usage: GPUBufferUsage.INDIRECT | GPUBufferUsage.COPY_DST,
        });
        
        const indirectData = new Uint32Array([
            18,  // indexCount
            1,   // instanceCount
            0,   // firstIndex
            0,   // baseVertex
            0    // firstInstance
        ]);
        this.device.queue.writeBuffer(this.indirectBuffer, 0, indirectData);
        
        console.log('[TestCube] Created cube with', this.totalVertices, 'vertices');
    }
    
    async generateMesh() {
        // No-op for test cube
    }
    
    getStats() {
        return {
            vertices: this.totalVertices,
            indices: this.totalIndices,
            triangles: Math.floor(this.totalIndices / 3),
        };
    }
}