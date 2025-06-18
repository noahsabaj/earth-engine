// Perlin Noise implementation for GPU terrain generation
// WGSL-compatible version using hash functions instead of permutation tables
// WGSL does not allow dynamic indexing of const arrays

// Gradient table for 3D Perlin noise
const GRAD3: array<vec3<f32>, 12> = array<vec3<f32>, 12>(
    vec3<f32>(1.0,1.0,0.0), vec3<f32>(-1.0,1.0,0.0), vec3<f32>(1.0,-1.0,0.0), vec3<f32>(-1.0,-1.0,0.0),
    vec3<f32>(1.0,0.0,1.0), vec3<f32>(-1.0,0.0,1.0), vec3<f32>(1.0,0.0,-1.0), vec3<f32>(-1.0,0.0,-1.0),
    vec3<f32>(0.0,1.0,1.0), vec3<f32>(0.0,-1.0,1.0), vec3<f32>(0.0,1.0,-1.0), vec3<f32>(0.0,-1.0,-1.0)
);

// Hash function to replace permutation table - WGSL compatible
fn hash_u32(x: u32) -> u32 {
    var h = x;
    h = h ^ (h >> 16u);
    h = h * 0x85ebca6bu;
    h = h ^ (h >> 13u);
    h = h * 0xc2b2ae35u;
    h = h ^ (h >> 16u);
    return h;
}

// 2D hash function
fn hash2d(x: u32, y: u32) -> u32 {
    return hash_u32(x + hash_u32(y));
}

// 3D hash function
fn hash3d(x: u32, y: u32, z: u32) -> u32 {
    return hash_u32(x + hash_u32(y + hash_u32(z)));
}

// Fade function for smooth interpolation
fn fade(t: f32) -> f32 {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

// Linear interpolation
fn lerp(t: f32, a: f32, b: f32) -> f32 {
    return a + t * (b - a);
}

// 2D Perlin noise - WGSL compatible version using hash functions
fn perlin2d(x: f32, y: f32) -> f32 {
    // Find unit square that contains point
    let X = u32(floor(x));
    let Y = u32(floor(y));
    
    // Find relative x,y of point in square
    let xf = fract(x);
    let yf = fract(y);
    
    // Compute fade curves
    let u = fade(xf);
    let v = fade(yf);
    
    // Hash coordinates of the 4 square corners using hash functions
    let hash_00 = hash2d(X, Y);
    let hash_10 = hash2d(X + 1u, Y);
    let hash_01 = hash2d(X, Y + 1u);
    let hash_11 = hash2d(X + 1u, Y + 1u);
    
    // And add blended results from 4 corners of square
    let res = lerp(v,
        lerp(u, grad2d(hash_00, xf, yf), grad2d(hash_10, xf - 1.0, yf)),
        lerp(u, grad2d(hash_01, xf, yf - 1.0), grad2d(hash_11, xf - 1.0, yf - 1.0))
    );
    
    return res;
}

// 2D gradient function
fn grad2d(hash: u32, x: f32, y: f32) -> f32 {
    let h = hash & 3u;
    let u = select(y, x, h < 2u);
    let v = select(x, y, h < 2u);
    return select(u, -u, (h & 1u) != 0u) + select(v, -v, ((h >> 1u) & 1u) != 0u);
}

// 3D Perlin noise - WGSL compatible version using hash functions
fn perlin3d(x: f32, y: f32, z: f32) -> f32 {
    // Find unit cube that contains point
    let X = u32(floor(x));
    let Y = u32(floor(y));
    let Z = u32(floor(z));
    
    // Find relative x,y,z of point in cube
    let xf = fract(x);
    let yf = fract(y);
    let zf = fract(z);
    
    // Compute fade curves
    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);
    
    // Hash coordinates of the 8 cube corners using hash functions
    let hash_000 = hash3d(X, Y, Z);
    let hash_100 = hash3d(X + 1u, Y, Z);
    let hash_010 = hash3d(X, Y + 1u, Z);
    let hash_110 = hash3d(X + 1u, Y + 1u, Z);
    let hash_001 = hash3d(X, Y, Z + 1u);
    let hash_101 = hash3d(X + 1u, Y, Z + 1u);
    let hash_011 = hash3d(X, Y + 1u, Z + 1u);
    let hash_111 = hash3d(X + 1u, Y + 1u, Z + 1u);
    
    // And add blended results from 8 corners of cube
    let res = lerp(w,
        lerp(v,
            lerp(u, grad3d(hash_000, xf, yf, zf), grad3d(hash_100, xf - 1.0, yf, zf)),
            lerp(u, grad3d(hash_010, xf, yf - 1.0, zf), grad3d(hash_110, xf - 1.0, yf - 1.0, zf))
        ),
        lerp(v,
            lerp(u, grad3d(hash_001, xf, yf, zf - 1.0), grad3d(hash_101, xf - 1.0, yf, zf - 1.0)),
            lerp(u, grad3d(hash_011, xf, yf - 1.0, zf - 1.0), grad3d(hash_111, xf - 1.0, yf - 1.0, zf - 1.0))
        )
    );
    
    return res;
}

// 3D gradient function
fn grad3d(hash: u32, x: f32, y: f32, z: f32) -> f32 {
    let h = hash & 15u;
    let u = select(y, x, h < 8u);
    let v = select(select(x, y, h < 4u), z, h < 2u || h == 12u || h == 14u);
    return select(u, -u, (h & 1u) != 0u) + select(v, -v, ((h >> 1u) & 1u) != 0u);
}

// Fractional Brownian Motion (fBm) for more natural terrain
fn fbm2d(x: f32, y: f32, octaves: i32, lacunarity: f32, persistence: f32) -> f32 {
    var value = 0.0;
    var amplitude = 1.0;
    var frequency = 1.0;
    var max_value = 0.0;
    
    for (var i = 0; i < octaves; i++) {
        value += perlin2d(x * frequency, y * frequency) * amplitude;
        max_value += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }
    
    return value / max_value;
}

// 3D fBm
fn fbm3d(x: f32, y: f32, z: f32, octaves: i32, lacunarity: f32, persistence: f32) -> f32 {
    var value = 0.0;
    var amplitude = 1.0;
    var frequency = 1.0;
    var max_value = 0.0;
    
    for (var i = 0; i < octaves; i++) {
        value += perlin3d(x * frequency, y * frequency, z * frequency) * amplitude;
        max_value += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }
    
    return value / max_value;
}

// Terrain height function
fn terrain_height(x: f32, z: f32) -> f32 {
    // Base terrain
    var height = fbm2d(x * 0.01, z * 0.01, 6, 2.0, 0.5) * 64.0;
    
    // Mountains
    let mountain = fbm2d(x * 0.005, z * 0.005, 4, 2.2, 0.45);
    if (mountain > 0.6) {
        height += (mountain - 0.6) * 200.0;
    }
    
    // Rivers (inverted ridged noise)
    let river = abs(perlin2d(x * 0.008, z * 0.008));
    if (river < 0.05) {
        height -= (0.05 - river) * 100.0;
    }
    
    return height + 64.0; // Base height at y=64
}

// Cave generation function
fn cave_density(x: f32, y: f32, z: f32) -> f32 {
    // 3D cave noise
    let cave1 = perlin3d(x * 0.05, y * 0.05, z * 0.05);
    let cave2 = perlin3d(x * 0.03, y * 0.03, z * 0.03);
    
    // Combine for more interesting caves
    return cave1 * cave1 + cave2 * 0.5;
}

// Ore distribution function
fn ore_density(x: f32, y: f32, z: f32, ore_type: u32) -> f32 {
    let scale = select(0.2, select(0.15, 0.1, ore_type == 2u), ore_type == 1u);
    let threshold = select(0.85, select(0.9, 0.95, ore_type == 2u), ore_type == 1u);
    
    let noise = perlin3d(x * scale, y * scale, z * scale);
    return select(0.0, 1.0, noise > threshold);
}