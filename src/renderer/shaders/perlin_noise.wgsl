// Perlin Noise implementation for GPU terrain generation
// Based on improved Perlin noise algorithm

// Permutation table - precomputed for performance
const PERM: array<u32, 512> = array<u32, 512>(
    151,160,137,91,90,15,131,13,201,95,96,53,194,233,7,225,140,36,103,30,69,142,8,99,37,240,21,10,23,
    190,6,148,247,120,234,75,0,26,197,62,94,252,219,203,117,35,11,32,57,177,33,88,237,149,56,87,174,20,
    125,136,171,168,68,175,74,165,71,134,139,48,27,166,77,146,158,231,83,111,229,122,60,211,133,230,220,
    105,92,41,55,46,245,40,244,102,143,54,65,25,63,161,1,216,80,73,209,76,132,187,208,89,18,169,200,196,
    135,130,116,188,159,86,164,100,109,198,173,186,3,64,52,217,226,250,124,123,5,202,38,147,118,126,255,
    82,85,212,207,206,59,227,47,16,58,17,182,189,28,42,223,183,170,213,119,248,152,2,44,154,163,70,221,
    153,101,155,167,43,172,9,129,22,39,253,19,98,108,110,79,113,224,232,178,185,112,104,218,246,97,228,
    251,34,242,193,238,210,144,12,191,179,162,241,81,51,145,235,249,14,239,107,49,192,214,31,181,199,
    106,157,184,84,204,176,115,121,50,45,127,4,150,254,138,236,205,93,222,114,67,29,24,72,243,141,128,
    195,78,66,215,61,156,180,
    // Repeat the sequence for seamless wrapping
    151,160,137,91,90,15,131,13,201,95,96,53,194,233,7,225,140,36,103,30,69,142,8,99,37,240,21,10,23,
    190,6,148,247,120,234,75,0,26,197,62,94,252,219,203,117,35,11,32,57,177,33,88,237,149,56,87,174,20,
    125,136,171,168,68,175,74,165,71,134,139,48,27,166,77,146,158,231,83,111,229,122,60,211,133,230,220,
    105,92,41,55,46,245,40,244,102,143,54,65,25,63,161,1,216,80,73,209,76,132,187,208,89,18,169,200,196,
    135,130,116,188,159,86,164,100,109,198,173,186,3,64,52,217,226,250,124,123,5,202,38,147,118,126,255,
    82,85,212,207,206,59,227,47,16,58,17,182,189,28,42,223,183,170,213,119,248,152,2,44,154,163,70,221,
    153,101,155,167,43,172,9,129,22,39,253,19,98,108,110,79,113,224,232,178,185,112,104,218,246,97,228,
    251,34,242,193,238,210,144,12,191,179,162,241,81,51,145,235,249,14,239,107,49,192,214,31,181,199,
    106,157,184,84,204,176,115,121,50,45,127,4,150,254,138,236,205,93,222,114,67,29,24,72,243,141,128,
    195,78,66,215,61,156,180
);

// Gradient table for 3D Perlin noise
const GRAD3: array<vec3<f32>, 12> = array<vec3<f32>, 12>(
    vec3<f32>(1.0,1.0,0.0), vec3<f32>(-1.0,1.0,0.0), vec3<f32>(1.0,-1.0,0.0), vec3<f32>(-1.0,-1.0,0.0),
    vec3<f32>(1.0,0.0,1.0), vec3<f32>(-1.0,0.0,1.0), vec3<f32>(1.0,0.0,-1.0), vec3<f32>(-1.0,0.0,-1.0),
    vec3<f32>(0.0,1.0,1.0), vec3<f32>(0.0,-1.0,1.0), vec3<f32>(0.0,1.0,-1.0), vec3<f32>(0.0,-1.0,-1.0)
);

// Fade function for smooth interpolation
fn fade(t: f32) -> f32 {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

// Linear interpolation
fn lerp(t: f32, a: f32, b: f32) -> f32 {
    return a + t * (b - a);
}

// 2D Perlin noise
fn perlin2d(x: f32, y: f32) -> f32 {
    // Find unit square that contains point
    let X = u32(floor(x)) & 255u;
    let Y = u32(floor(y)) & 255u;
    
    // Find relative x,y of point in square
    let xf = fract(x);
    let yf = fract(y);
    
    // Compute fade curves
    let u = fade(xf);
    let v = fade(yf);
    
    // Hash coordinates of the 4 square corners
    let A = PERM[X] + Y;
    let B = PERM[X + 1u] + Y;
    
    // And add blended results from 4 corners of square
    let res = lerp(v,
        lerp(u, grad2d(PERM[A], xf, yf), grad2d(PERM[B], xf - 1.0, yf)),
        lerp(u, grad2d(PERM[A + 1u], xf, yf - 1.0), grad2d(PERM[B + 1u], xf - 1.0, yf - 1.0))
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

// 3D Perlin noise
fn perlin3d(x: f32, y: f32, z: f32) -> f32 {
    // Find unit cube that contains point
    let X = u32(floor(x)) & 255u;
    let Y = u32(floor(y)) & 255u;
    let Z = u32(floor(z)) & 255u;
    
    // Find relative x,y,z of point in cube
    let xf = fract(x);
    let yf = fract(y);
    let zf = fract(z);
    
    // Compute fade curves
    let u = fade(xf);
    let v = fade(yf);
    let w = fade(zf);
    
    // Hash coordinates of the 8 cube corners
    let A = PERM[X] + Y;
    let AA = PERM[A] + Z;
    let AB = PERM[A + 1u] + Z;
    let B = PERM[X + 1u] + Y;
    let BA = PERM[B] + Z;
    let BB = PERM[B + 1u] + Z;
    
    // And add blended results from 8 corners of cube
    let res = lerp(w,
        lerp(v,
            lerp(u, grad3d(PERM[AA], xf, yf, zf), grad3d(PERM[BA], xf - 1.0, yf, zf)),
            lerp(u, grad3d(PERM[AB], xf, yf - 1.0, zf), grad3d(PERM[BB], xf - 1.0, yf - 1.0, zf))
        ),
        lerp(v,
            lerp(u, grad3d(PERM[AA + 1u], xf, yf, zf - 1.0), grad3d(PERM[BA + 1u], xf - 1.0, yf, zf - 1.0)),
            lerp(u, grad3d(PERM[AB + 1u], xf, yf - 1.0, zf - 1.0), grad3d(PERM[BB + 1u], xf - 1.0, yf - 1.0, zf - 1.0))
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