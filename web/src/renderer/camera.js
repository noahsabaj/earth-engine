// Camera System - First-person camera matching Rust's data_camera
// Pure functional approach with immutable data

import { Vec3, Mat4, clamp, degToRad } from '../core/math.js';

export class CameraData {
    constructor(options = {}) {
        this.position = options.position || [0, 100, 10];
        this.yaw = options.yaw || 0; // radians
        this.pitch = options.pitch || 0; // radians
        this.fov = options.fov || degToRad(70);
        this.aspect = options.aspect || 16/9;
        this.near = options.near || 0.1;
        this.far = options.far || 1000;
    }
    
    // Calculate forward vector from yaw/pitch
    getForward() {
        const cosPitch = Math.cos(this.pitch);
        return new Vec3(
            Math.sin(this.yaw) * cosPitch,
            -Math.sin(this.pitch),
            Math.cos(this.yaw) * cosPitch
        ).normalize();
    }
    
    getRight() {
        return new Vec3(
            Math.cos(this.yaw),
            0,
            -Math.sin(this.yaw)
        ).normalize();
    }
    
    getUp() {
        return this.getRight().cross(this.getForward());
    }
    
    // Get view matrix
    getViewMatrix() {
        const pos = Vec3.from(this.position);
        const forward = this.getForward();
        const target = pos.add(forward);
        
        return Mat4.lookAt(pos, target, Vec3.UP);
    }
    
    // Get projection matrix
    getProjectionMatrix() {
        return Mat4.perspective(this.fov, this.aspect, this.near, this.far);
    }
    
    // Get combined view-projection matrix
    getViewProjectionMatrix() {
        return this.getProjectionMatrix().mul(this.getViewMatrix());
    }
    
    // Create GPU buffer data
    toGPUData() {
        const view = this.getViewMatrix();
        const proj = this.getProjectionMatrix();
        const viewProj = proj.mul(view);
        
        // Pack into Float32Array for GPU
        const data = new Float32Array(16 + 16 + 16 + 4); // view + proj + viewProj + position
        
        data.set(view.data, 0);
        data.set(proj.data, 16);
        data.set(viewProj.data, 32);
        data.set([...this.position, 1.0], 48);
        
        return data;
    }
}

// Camera controller for input handling
export class CameraController {
    constructor(camera) {
        this.camera = camera;
        this.velocity = new Vec3();
        this.mouseSensitivity = 0.002;
        this.moveSpeed = 50;
        this.sprintMultiplier = 2;
        this.jumpVelocity = 15;
        
        this.keys = new Set();
        this.mouseX = 0;
        this.mouseY = 0;
        this.mouseDeltaX = 0;
        this.mouseDeltaY = 0;
        this.isPointerLocked = false;
        
        this.setupEventListeners();
    }
    
    setupEventListeners() {
        // Keyboard
        window.addEventListener('keydown', (e) => {
            this.keys.add(e.code);
            
            // Request pointer lock on first interaction
            if (!this.isPointerLocked && document.pointerLockElement === null) {
                document.body.requestPointerLock();
            }
        });
        
        window.addEventListener('keyup', (e) => {
            this.keys.delete(e.code);
        });
        
        // Mouse
        window.addEventListener('mousemove', (e) => {
            if (document.pointerLockElement) {
                this.mouseDeltaX += e.movementX;
                this.mouseDeltaY += e.movementY;
            }
        });
        
        // Pointer lock
        document.addEventListener('pointerlockchange', () => {
            this.isPointerLocked = document.pointerLockElement !== null;
            if (!this.isPointerLocked) {
                this.mouseDeltaX = 0;
                this.mouseDeltaY = 0;
            }
        });
        
        // Click to lock
        document.body.addEventListener('click', () => {
            if (!this.isPointerLocked) {
                document.body.requestPointerLock();
            }
        });
    }
    
    update(deltaTime) {
        // Mouse look
        if (this.isPointerLocked) {
            this.camera.yaw -= this.mouseDeltaX * this.mouseSensitivity;
            this.camera.pitch -= this.mouseDeltaY * this.mouseSensitivity;
            
            // Clamp pitch to prevent flipping
            this.camera.pitch = clamp(this.camera.pitch, -Math.PI/2 + 0.01, Math.PI/2 - 0.01);
            
            this.mouseDeltaX = 0;
            this.mouseDeltaY = 0;
        }
        
        // Movement
        const forward = this.camera.getForward();
        const right = this.camera.getRight();
        
        let moveSpeed = this.moveSpeed;
        if (this.keys.has('ShiftLeft')) {
            moveSpeed *= this.sprintMultiplier;
        }
        
        const movement = new Vec3();
        
        if (this.keys.has('KeyW')) {
            movement.x += forward.x;
            movement.z += forward.z;
        }
        if (this.keys.has('KeyS')) {
            movement.x -= forward.x;
            movement.z -= forward.z;
        }
        if (this.keys.has('KeyA')) {
            movement.x -= right.x;
            movement.z -= right.z;
        }
        if (this.keys.has('KeyD')) {
            movement.x += right.x;
            movement.z += right.z;
        }
        
        // Normalize diagonal movement
        if (movement.length() > 0) {
            const normalized = movement.normalize();
            this.velocity.x = normalized.x * moveSpeed;
            this.velocity.z = normalized.z * moveSpeed;
        } else {
            // Deceleration
            this.velocity.x *= 0.9;
            this.velocity.z *= 0.9;
        }
        
        // Vertical movement
        if (this.keys.has('Space')) {
            this.velocity.y = this.jumpVelocity;
        }
        if (this.keys.has('ControlLeft')) {
            this.velocity.y = -this.jumpVelocity;
        }
        
        // Apply gravity (simplified - no collision)
        this.velocity.y -= 30 * deltaTime;
        
        // Ground check (simplified) - ground is at y=50
        if (this.camera.position[1] <= 52 && this.velocity.y < 0) {
            this.camera.position[1] = 52;  // Keep camera 2 units above ground
            this.velocity.y = 0;
        }
        
        // Update position
        this.camera.position[0] += this.velocity.x * deltaTime;
        this.camera.position[1] += this.velocity.y * deltaTime;
        this.camera.position[2] += this.velocity.z * deltaTime;
    }
    
    // Get debug info
    getDebugInfo() {
        return {
            position: this.camera.position.map(v => v.toFixed(1)),
            yaw: (this.camera.yaw * 180 / Math.PI).toFixed(1),
            pitch: (this.camera.pitch * 180 / Math.PI).toFixed(1),
            velocity: [this.velocity.x.toFixed(1), this.velocity.y.toFixed(1), this.velocity.z.toFixed(1)],
        };
    }
}