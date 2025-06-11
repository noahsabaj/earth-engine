// Earth Engine - Data-Oriented JavaScript Implementation
// Pure functions and data structures, no classes

import { engine } from './engine.js';

// Initialize engine function
export async function initializeEngine(canvas) {
    console.log('[Index] Initializing Earth Engine (Data-Oriented)...');
    
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

// Auto-initialize if canvas exists
if (typeof window !== 'undefined') {
    window.addEventListener('DOMContentLoaded', async () => {
        const canvas = document.getElementById('canvas');
        if (canvas) {
            console.log('[Index] Found canvas, auto-initializing...');
            try {
                await initializeEngine(canvas);
            } catch (error) {
                console.error('[Index] Auto-initialization failed:', error);
            }
        }
    });
}