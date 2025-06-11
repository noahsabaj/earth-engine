// Shader Loader - Loads and caches WGSL shaders
// These are the SAME shaders used by the Rust engine

export class ShaderLoader {
    constructor(basePath = '../earth-engine/src/renderer/shaders/') {
        this.basePath = basePath;
        this.cache = new Map();
        this.pending = new Map();
    }
    
    // Load a shader file
    async load(filename) {
        // Return cached if available
        if (this.cache.has(filename)) {
            return this.cache.get(filename);
        }
        
        // Wait for pending load if in progress
        if (this.pending.has(filename)) {
            return this.pending.get(filename);
        }
        
        // Start new load
        const loadPromise = this._loadShader(filename);
        this.pending.set(filename, loadPromise);
        
        try {
            const shader = await loadPromise;
            this.cache.set(filename, shader);
            this.pending.delete(filename);
            return shader;
        } catch (error) {
            this.pending.delete(filename);
            throw error;
        }
    }
    
    async _loadShader(filename) {
        const url = this.basePath + filename;
        console.log(`[Shader] Loading ${filename}...`);
        
        try {
            const response = await fetch(url);
            if (!response.ok) {
                throw new Error(`Failed to load shader: ${response.statusText}`);
            }
            
            let code = await response.text();
            
            // Process includes
            code = await this._processIncludes(code, filename);
            
            console.log(`[Shader] Loaded ${filename} (${code.length} bytes)`);
            return code;
        } catch (error) {
            console.error(`[Shader] Failed to load ${filename}:`, error);
            throw error;
        }
    }
    
    // Process #include directives in shaders
    async _processIncludes(code, parentFile) {
        const includeRegex = /#include\s+"([^"]+)"/g;
        const includes = [];
        let match;
        
        while ((match = includeRegex.exec(code)) !== null) {
            includes.push({
                full: match[0],
                file: match[1],
                index: match.index
            });
        }
        
        // Process includes in reverse order to maintain indices
        for (let i = includes.length - 1; i >= 0; i--) {
            const inc = includes[i];
            const includePath = this._resolveIncludePath(inc.file, parentFile);
            const includeCode = await this.load(includePath);
            
            code = code.substring(0, inc.index) + 
                   includeCode + 
                   code.substring(inc.index + inc.full.length);
        }
        
        return code;
    }
    
    _resolveIncludePath(includePath, parentFile) {
        // Handle relative paths
        if (includePath.startsWith('./')) {
            const parentDir = parentFile.substring(0, parentFile.lastIndexOf('/') + 1);
            return parentDir + includePath.substring(2);
        }
        return includePath;
    }
    
    // Load common shaders
    async loadCommonShaders() {
        const shaders = {
            // Terrain generation
            terrainGen: await this.load('perlin_noise.wgsl'),
            
            // Rendering
            voxelVert: await this.load('voxel.wgsl'),
            
            // Compute shaders
            meshGen: await this.load('chunk_compute.wgsl'),
            
            // GPU culling
            frustumCull: await this.load('gpu_culling.wgsl'),
        };
        
        return shaders;
    }
    
    // Create shader with common functions injected
    createShaderWithCommon(mainCode, commonCode = '') {
        return commonCode + '\n\n' + mainCode;
    }
    
    // Clear cache
    clearCache() {
        this.cache.clear();
        console.log('[Shader] Cache cleared');
    }
    
    // Get cache stats
    getCacheStats() {
        return {
            cached: this.cache.size,
            pending: this.pending.size,
            totalSize: Array.from(this.cache.values())
                .reduce((sum, code) => sum + code.length, 0)
        };
    }
}

// Shader code snippets for when files aren't available
export const BUILTIN_SHADERS = {
    // Morton encoding function
    mortonEncode: `
        fn morton_encode_3d(x: u32, y: u32, z: u32) -> u32 {
            // 32-bit Morton encoding for 10-bit coordinates (up to 1024)
            var xx = x & 0x3FFu; // 10 bits
            var yy = y & 0x3FFu;
            var zz = z & 0x3FFu;
            
            // Spread bits - adapted for 32-bit arithmetic
            xx = (xx | (xx << 16u)) & 0x030000FFu;
            xx = (xx | (xx << 8u))  & 0x0300F00Fu;
            xx = (xx | (xx << 4u))  & 0x030C30C3u;
            xx = (xx | (xx << 2u))  & 0x09249249u;
            
            yy = (yy | (yy << 16u)) & 0x030000FFu;
            yy = (yy | (yy << 8u))  & 0x0300F00Fu;
            yy = (yy | (yy << 4u))  & 0x030C30C3u;
            yy = (yy | (yy << 2u))  & 0x09249249u;
            
            zz = (zz | (zz << 16u)) & 0x030000FFu;
            zz = (zz | (zz << 8u))  & 0x0300F00Fu;
            zz = (zz | (zz << 4u))  & 0x030C30C3u;
            zz = (zz | (zz << 2u))  & 0x09249249u;
            
            return xx | (yy << 1u) | (zz << 2u);
        }
    `,
    
    // Simple noise function
    noise3d: `
        fn hash(p: vec3<f32>) -> f32 {
            var p3 = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
            p3 += dot(p3, p3.yxz + 33.33);
            return fract((p3.x + p3.y) * p3.z);
        }
        
        fn noise3d(p: vec3<f32>) -> f32 {
            let i = floor(p);
            let f = fract(p);
            let u = f * f * (3.0 - 2.0 * f);
            
            return mix(
                mix(mix(hash(i + vec3<f32>(0,0,0)), hash(i + vec3<f32>(1,0,0)), u.x),
                    mix(hash(i + vec3<f32>(0,1,0)), hash(i + vec3<f32>(1,1,0)), u.x), u.y),
                mix(mix(hash(i + vec3<f32>(0,0,1)), hash(i + vec3<f32>(1,0,1)), u.x),
                    mix(hash(i + vec3<f32>(0,1,1)), hash(i + vec3<f32>(1,1,1)), u.x), u.y),
                u.z
            );
        }
    `
};