// Earth Engine - Data-Oriented JavaScript Implementation
// Pure functions and data structures, no classes

console.log('[Index] Loading index.js module...');

import { 
    initializeEngine as engineInit, 
    startEngine,
    engineState, gpuState, worldState, cameraState, meshState, rendererState 
} from './engine.js';

console.log('[Index] Engine functions imported');

// Re-export the initialize function with our wrapper
export async function initializeEngine(canvas) {
    console.log('[Index] initializeEngine called with canvas:', canvas);
    
    try {
        // Set canvas size
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;
        
        // Initialize all systems
        await engineInit(canvas);
        
        // Start render loop
        startEngine();
        
        // Handle window resize
        window.addEventListener('resize', () => {
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;
        });
        
        console.log('[Index] Engine started successfully');
        console.log('[Index] Access state via window.earthEngineState');
        
        // Expose states to window for debugging (pure data)
        window.earthEngineState = {
            engine: engineState,
            gpu: gpuState,
            world: worldState,
            camera: cameraState,
            mesh: meshState,
            renderer: rendererState
        };
        
        return true; // Return success instead of object
    } catch (error) {
        console.error('[Index] Failed to initialize engine:', error);
        throw error;
    }
}

// No auto-initialization - let index.html handle it

console.log('[Index] Module loaded, exporting initializeEngine:', initializeEngine);