// Earth Engine - Data-Oriented JavaScript Implementation
// Pure functions and data structures, no classes

console.log('[Index] Loading index.js module...');

import { engine } from './engine.js';

console.log('[Index] Engine imported:', engine);

// Initialize engine function
export async function initializeEngine(canvas) {
    console.log('[Index] initializeEngine called with canvas:', canvas);
    
    try {
        // Set canvas size
        canvas.width = window.innerWidth;
        canvas.height = window.innerHeight;
        
        // Initialize all systems
        await engine.initialize(canvas);
        
        // Start render loop
        engine.start();
        
        // Handle window resize
        window.addEventListener('resize', () => {
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;
        });
        
        console.log('[Index] Engine started successfully');
        console.log('[Index] Access state via window.earthEngine.state');
        
        // Expose engine to window for debugging
        window.earthEngine = engine;
        
        return engine;
    } catch (error) {
        console.error('[Index] Failed to initialize engine:', error);
        throw error;
    }
}

// No auto-initialization - let index.html handle it

console.log('[Index] Module loaded, exporting initializeEngine:', initializeEngine);