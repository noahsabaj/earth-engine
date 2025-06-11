// Camera State - Pure data and functions for camera management
// No classes, just data transformations

import { createBuffer, writeBuffer } from './gpu-state.js';

// Camera state - pure data
export const cameraState = {
    // Transform data
    position: new Float32Array([10, 60, 30]), // Look at small test plane
    rotation: new Float32Array([0, -0.5]), // yaw, pitch - look down
    
    // Projection parameters
    fov: Math.PI / 3,
    aspect: 1.0,
    near: 0.1,
    far: 1000.0,
    
    // Matrices (column-major for GPU)
    matrices: {
        view: new Float32Array(16),
        projection: new Float32Array(16),
        viewProjection: new Float32Array(16)
    },
    
    // GPU buffer for uniforms
    buffer: null,
    gpuData: new ArrayBuffer(256), // Padded for alignment
    
    // Input state
    input: {
        keys: new Set(),
        mouse: { x: 0, y: 0, deltaX: 0, deltaY: 0 },
        pointerLocked: false
    },
    
    // Movement parameters
    moveSpeed: 50.0,
    lookSpeed: 0.003,
    
    initialized: false
};

// Initialize camera
export function initializeCamera(canvas) {
    console.log('[Camera] Initializing...');
    
    // Set aspect ratio
    cameraState.aspect = canvas.width / canvas.height;
    
    // Create GPU buffer
    cameraState.buffer = createBuffer(
        256, // Padded size for alignment
        GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
        'CameraUniforms'
    );
    
    // Setup input handlers
    setupInputHandlers(canvas);
    
    // Initial update
    updateCameraMatrices();
    
    cameraState.initialized = true;
    console.log('[Camera] Initialized');
}

// Matrix math functions (pure)
export function createIdentityMatrix() {
    const m = new Float32Array(16);
    m[0] = m[5] = m[10] = m[15] = 1;
    return m;
}

export function createPerspectiveMatrix(fov, aspect, near, far) {
    const f = 1.0 / Math.tan(fov / 2);
    const rangeInv = 1 / (near - far);
    
    return new Float32Array([
        f / aspect, 0, 0, 0,
        0, f, 0, 0,
        0, 0, (near + far) * rangeInv, -1,
        0, 0, near * far * rangeInv * 2, 0
    ]);
}

export function createLookAtMatrix(eye, target, up) {
    const zAxis = normalize(subtract(eye, target));
    const xAxis = normalize(cross(up, zAxis));
    const yAxis = normalize(cross(zAxis, xAxis));
    
    return new Float32Array([
        xAxis[0], yAxis[0], zAxis[0], 0,
        xAxis[1], yAxis[1], zAxis[1], 0,
        xAxis[2], yAxis[2], zAxis[2], 0,
        -dot(xAxis, eye), -dot(yAxis, eye), -dot(zAxis, eye), 1
    ]);
}

export function multiplyMatrices(a, b) {
    const result = new Float32Array(16);
    for (let i = 0; i < 4; i++) {
        for (let j = 0; j < 4; j++) {
            let sum = 0;
            for (let k = 0; k < 4; k++) {
                sum += a[k * 4 + j] * b[i * 4 + k];
            }
            result[i * 4 + j] = sum;
        }
    }
    return result;
}

// Vector math helpers
function normalize(v) {
    const len = Math.sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
    return new Float32Array([v[0] / len, v[1] / len, v[2] / len]);
}

function subtract(a, b) {
    return new Float32Array([a[0] - b[0], a[1] - b[1], a[2] - b[2]]);
}

function cross(a, b) {
    return new Float32Array([
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0]
    ]);
}

function dot(a, b) {
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

// Update camera matrices
export function updateCameraMatrices() {
    const yaw = cameraState.rotation[0];
    const pitch = cameraState.rotation[1];
    
    // Calculate forward vector (negative Z is forward)
    const forward = new Float32Array([
        -Math.sin(yaw) * Math.cos(pitch),
        Math.sin(pitch),
        -Math.cos(yaw) * Math.cos(pitch)
    ]);
    
    // Calculate target
    const target = new Float32Array([
        cameraState.position[0] + forward[0],
        cameraState.position[1] + forward[1],
        cameraState.position[2] + forward[2]
    ]);
    
    // Update matrices
    cameraState.matrices.view = createLookAtMatrix(
        cameraState.position,
        target,
        new Float32Array([0, 1, 0])
    );
    
    cameraState.matrices.projection = createPerspectiveMatrix(
        cameraState.fov,
        cameraState.aspect,
        cameraState.near,
        cameraState.far
    );
    
    cameraState.matrices.viewProjection = multiplyMatrices(
        cameraState.matrices.projection,
        cameraState.matrices.view
    );
}

// Update camera from input
export function updateCamera(deltaTime) {
    let moved = false;
    
    // Rotation from mouse
    if (cameraState.input.pointerLocked) {
        cameraState.rotation[0] -= cameraState.input.mouse.deltaX * cameraState.lookSpeed; // Inverted X
        cameraState.rotation[1] = Math.max(-Math.PI/2, Math.min(Math.PI/2, 
            cameraState.rotation[1] - cameraState.input.mouse.deltaY * cameraState.lookSpeed)); // Inverted Y
        
        cameraState.input.mouse.deltaX = 0;
        cameraState.input.mouse.deltaY = 0;
        moved = true;
    }
    
    // Movement from keyboard
    const moveSpeed = cameraState.moveSpeed * deltaTime;
    const yaw = cameraState.rotation[0];
    const pitch = cameraState.rotation[1];
    
    // Calculate movement vectors
    const forward = new Float32Array([
        -Math.sin(yaw) * Math.cos(pitch),
        0, // Don't move vertically with forward/back
        -Math.cos(yaw) * Math.cos(pitch)
    ]);
    
    const right = new Float32Array([
        Math.cos(yaw),
        0,
        -Math.sin(yaw)
    ]);
    
    // Apply movement
    if (cameraState.input.keys.has('KeyW')) {
        cameraState.position[0] += forward[0] * moveSpeed;
        cameraState.position[1] += forward[1] * moveSpeed;
        cameraState.position[2] += forward[2] * moveSpeed;
        moved = true;
    }
    if (cameraState.input.keys.has('KeyS')) {
        cameraState.position[0] -= forward[0] * moveSpeed;
        cameraState.position[1] -= forward[1] * moveSpeed;
        cameraState.position[2] -= forward[2] * moveSpeed;
        moved = true;
    }
    if (cameraState.input.keys.has('KeyA')) {
        cameraState.position[0] -= right[0] * moveSpeed;
        cameraState.position[2] -= right[2] * moveSpeed;
        moved = true;
    }
    if (cameraState.input.keys.has('KeyD')) {
        cameraState.position[0] += right[0] * moveSpeed;
        cameraState.position[2] += right[2] * moveSpeed;
        moved = true;
    }
    if (cameraState.input.keys.has('Space')) {
        cameraState.position[1] += moveSpeed;
        moved = true;
    }
    if (cameraState.input.keys.has('ShiftLeft')) {
        cameraState.position[1] -= moveSpeed;
        moved = true;
    }
    
    if (moved) {
        updateCameraMatrices();
    }
    
    return moved;
}

// Get GPU data for uniforms
export function getCameraGPUData() {
    const dataView = new DataView(cameraState.gpuData);
    let offset = 0;
    
    // Write view matrix
    for (let i = 0; i < 16; i++) {
        dataView.setFloat32(offset, cameraState.matrices.view[i], true);
        offset += 4;
    }
    
    // Write projection matrix
    for (let i = 0; i < 16; i++) {
        dataView.setFloat32(offset, cameraState.matrices.projection[i], true);
        offset += 4;
    }
    
    // Write viewProjection matrix
    for (let i = 0; i < 16; i++) {
        dataView.setFloat32(offset, cameraState.matrices.viewProjection[i], true);
        offset += 4;
    }
    
    // Write position
    dataView.setFloat32(offset, cameraState.position[0], true);
    dataView.setFloat32(offset + 4, cameraState.position[1], true);
    dataView.setFloat32(offset + 8, cameraState.position[2], true);
    dataView.setFloat32(offset + 12, 1.0, true); // w component
    
    return cameraState.gpuData;
}

// Upload camera data to GPU
export function uploadCameraData() {
    const gpuData = getCameraGPUData();
    writeBuffer(cameraState.buffer, 0, gpuData);
}

// Setup input handlers
function setupInputHandlers(canvas) {
    // Keyboard
    window.addEventListener('keydown', (e) => {
        cameraState.input.keys.add(e.code);
    });
    
    window.addEventListener('keyup', (e) => {
        cameraState.input.keys.delete(e.code);
    });
    
    // Mouse
    canvas.addEventListener('click', () => {
        canvas.requestPointerLock();
    });
    
    document.addEventListener('pointerlockchange', () => {
        cameraState.input.pointerLocked = document.pointerLockElement === canvas;
    });
    
    window.addEventListener('mousemove', (e) => {
        if (cameraState.input.pointerLocked) {
            cameraState.input.mouse.deltaX += e.movementX;
            cameraState.input.mouse.deltaY += e.movementY;
        }
    });
}