// Math utilities - Vectors, Matrices, and common operations
// Matches the patterns from Rust's cgmath/glam

export class Vec3 {
    constructor(x = 0, y = 0, z = 0) {
        this.x = x;
        this.y = y;
        this.z = z;
    }
    
    static from(arr) {
        return new Vec3(arr[0], arr[1], arr[2]);
    }
    
    toArray() {
        return [this.x, this.y, this.z];
    }
    
    add(other) {
        return new Vec3(this.x + other.x, this.y + other.y, this.z + other.z);
    }
    
    sub(other) {
        return new Vec3(this.x - other.x, this.y - other.y, this.z - other.z);
    }
    
    mul(scalar) {
        return new Vec3(this.x * scalar, this.y * scalar, this.z * scalar);
    }
    
    dot(other) {
        return this.x * other.x + this.y * other.y + this.z * other.z;
    }
    
    cross(other) {
        return new Vec3(
            this.y * other.z - this.z * other.y,
            this.z * other.x - this.x * other.z,
            this.x * other.y - this.y * other.x
        );
    }
    
    length() {
        return Math.sqrt(this.x * this.x + this.y * this.y + this.z * this.z);
    }
    
    normalize() {
        const len = this.length();
        if (len === 0) return new Vec3();
        return this.mul(1 / len);
    }
    
    static UP = new Vec3(0, 1, 0);
    static FORWARD = new Vec3(0, 0, -1);
    static RIGHT = new Vec3(1, 0, 0);
}

export class Mat4 {
    constructor(data = null) {
        this.data = data || new Float32Array([
            1, 0, 0, 0,
            0, 1, 0, 0,
            0, 0, 1, 0,
            0, 0, 0, 1
        ]);
    }
    
    static identity() {
        return new Mat4();
    }
    
    static perspective(fovRadians, aspect, near, far) {
        const f = 1.0 / Math.tan(fovRadians / 2);
        const nf = 1 / (near - far);
        
        return new Mat4(new Float32Array([
            f / aspect, 0, 0, 0,
            0, f, 0, 0,
            0, 0, (far + near) * nf, -1,
            0, 0, 2 * far * near * nf, 0
        ]));
    }
    
    static lookAt(eye, target, up) {
        const zAxis = eye.sub(target).normalize();
        const xAxis = up.cross(zAxis).normalize();
        const yAxis = zAxis.cross(xAxis);
        
        return new Mat4(new Float32Array([
            xAxis.x, xAxis.y, xAxis.z, 0,
            yAxis.x, yAxis.y, yAxis.z, 0,
            zAxis.x, zAxis.y, zAxis.z, 0,
            -xAxis.dot(eye), -yAxis.dot(eye), -zAxis.dot(eye), 1
        ]));
    }
    
    static translation(x, y, z) {
        return new Mat4(new Float32Array([
            1, 0, 0, 0,
            0, 1, 0, 0,
            0, 0, 1, 0,
            x, y, z, 1
        ]));
    }
    
    static rotationY(radians) {
        const c = Math.cos(radians);
        const s = Math.sin(radians);
        
        return new Mat4(new Float32Array([
            c, 0, s, 0,
            0, 1, 0, 0,
            -s, 0, c, 0,
            0, 0, 0, 1
        ]));
    }
    
    mul(other) {
        const a = this.data;
        const b = other.data;
        const result = new Float32Array(16);
        
        for (let i = 0; i < 4; i++) {
            for (let j = 0; j < 4; j++) {
                let sum = 0;
                for (let k = 0; k < 4; k++) {
                    sum += a[i * 4 + k] * b[k * 4 + j];
                }
                result[i * 4 + j] = sum;
            }
        }
        
        return new Mat4(result);
    }
    
    toArray() {
        return Array.from(this.data);
    }
}

// Common math functions
export function degToRad(degrees) {
    return degrees * Math.PI / 180;
}

export function radToDeg(radians) {
    return radians * 180 / Math.PI;
}

export function clamp(value, min, max) {
    return Math.max(min, Math.min(max, value));
}

export function lerp(a, b, t) {
    return a + (b - a) * t;
}

// Chunk position helpers (matching Rust)
export class ChunkPos {
    constructor(x, y, z) {
        this.x = x;
        this.y = y;
        this.z = z;
    }
    
    static fromWorldPos(worldX, worldY, worldZ, chunkSize = 32) {
        return new ChunkPos(
            Math.floor(worldX / chunkSize),
            Math.floor(worldY / chunkSize),
            Math.floor(worldZ / chunkSize)
        );
    }
    
    toWorldPos(chunkSize = 32) {
        return new Vec3(
            this.x * chunkSize,
            this.y * chunkSize,
            this.z * chunkSize
        );
    }
    
    toIndex(chunksPerAxis) {
        return this.x + this.y * chunksPerAxis + this.z * chunksPerAxis * chunksPerAxis;
    }
    
    equals(other) {
        return this.x === other.x && this.y === other.y && this.z === other.z;
    }
}