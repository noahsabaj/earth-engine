// Weather Compute Shader
// Handles weather transitions, particle simulation, and atmospheric effects

// Constants from engine - TODO: These should come from generated constants
const DEFAULT_HUMIDITY: u32 = 500u;
const DEFAULT_WIND_SPEED: u32 = 50u;
const MAX_HUMIDITY: u32 = 5000u;

struct WeatherData {
    weather_type_intensity: u32,
    temperature: i32, // Actually i16 but WGSL doesn't have i16
    humidity: u32,    // Actually u16
    wind_speed: u32,  // Actually u16
    wind_direction: u32, // Actually u16
    visibility: u32,  // Actually u16
    precipitation_rate: u32, // Actually u16
}

struct WeatherTransition {
    current: WeatherData,
    target_weather: WeatherData,
    progress: u32, // Actually u16
    speed: u32,    // Actually u16
    change_timer: u32,
    biome_type: u32,
}

struct PrecipitationParticle {
    position: vec3<f32>,
    particle_type: u32,
    velocity: vec3<f32>,
    lifetime: f32,
    size: f32,
    _padding: vec3<f32>,
}

struct WeatherConfig {
    frame_number: u32,
    delta_time_ms: u32,
    player_position: vec3<f32>,
    precipitation_radius: f32,
    max_particles: u32,
    particle_count: u32,
    random_seed: u32,
    flags: u32,
}

// Weather types
const WEATHER_CLEAR: u32 = 0u;
const WEATHER_CLOUDY: u32 = 1u;
const WEATHER_RAIN: u32 = 2u;
const WEATHER_SNOW: u32 = 3u;
const WEATHER_THUNDERSTORM: u32 = 4u;
const WEATHER_FOG: u32 = 5u;
const WEATHER_SANDSTORM: u32 = 6u;

// Intensity levels
const INTENSITY_NONE: u32 = 0u;
const INTENSITY_LIGHT: u32 = 1u;
const INTENSITY_MODERATE: u32 = 2u;
const INTENSITY_HEAVY: u32 = 3u;
const INTENSITY_EXTREME: u32 = 4u;

// Biome types
const BIOME_PLAINS: u32 = 0u;
const BIOME_FOREST: u32 = 1u;
const BIOME_DESERT: u32 = 2u;
const BIOME_TUNDRA: u32 = 3u;
const BIOME_MOUNTAIN: u32 = 4u;
const BIOME_SWAMP: u32 = 5u;
const BIOME_OCEAN: u32 = 6u;

// Bind groups
@group(0) @binding(0) var<storage, read_write> weather_data: array<WeatherData>;
@group(0) @binding(1) var<storage, read_write> transitions: array<WeatherTransition>;
@group(0) @binding(2) var<storage, read_write> particles: array<PrecipitationParticle>;
@group(0) @binding(3) var<uniform> config: WeatherConfig;

// Workgroup shared memory for particle spawning coordination
var<workgroup> spawn_count: atomic<u32>;
var<workgroup> next_particle_idx: atomic<u32>;

// Simple hash function for randomness
fn hash(p: u32) -> u32 {
    var x = p;
    x = x ^ (x >> 16u);
    x = x * 0x85ebca6bu;
    x = x ^ (x >> 13u);
    x = x * 0xc2b2ae35u;
    x = x ^ (x >> 16u);
    return x;
}

// Generate random float between 0 and 1
fn random(seed: u32) -> f32 {
    return f32(hash(seed)) / 4294967295.0;
}

// Get default weather for biome
fn get_biome_weather(biome: u32, variant: u32) -> WeatherData {
    var weather: WeatherData;
    
    if (biome == BIOME_DESERT) {
        if (variant < 3u) {
            // Clear weather (most common)
            weather.weather_type_intensity = WEATHER_CLEAR;
            weather.temperature = 350; // 35°C
            weather.humidity = 1000; // 10%
            weather.wind_speed = 150; // 15 m/s
            weather.wind_direction = 45u;
            weather.visibility = 1000u;
            weather.precipitation_rate = 0u;
        } else {
            // Sandstorm
            weather.weather_type_intensity = WEATHER_SANDSTORM | (INTENSITY_MODERATE << 8u);
            weather.temperature = 400; // 40°C
            weather.humidity = DEFAULT_HUMIDITY; // 5%
            weather.wind_speed = 300; // 30 m/s
            weather.wind_direction = 90u;
            weather.visibility = 300u; // 0.3
            weather.precipitation_rate = 0u;
        }
    } else if (biome == BIOME_TUNDRA) {
        // Snow variations
        let intensity = min(variant, 3u) + 1u; // Light to Extreme
        weather.weather_type_intensity = WEATHER_SNOW | (intensity << 8u);
        weather.temperature = -50; // -5°C
        weather.humidity = 7000; // 70%
        weather.wind_speed = 100; // 10 m/s
        weather.wind_direction = 270u;
        weather.visibility = 1000u - (intensity * 200u);
        weather.precipitation_rate = intensity * 100u;
    } else if (biome == BIOME_FOREST) {
        if (variant < 2u) {
            // Clear or light fog
            weather.weather_type_intensity = select(WEATHER_CLEAR, WEATHER_FOG | (INTENSITY_LIGHT << 8u), variant == 1u);
            weather.temperature = 180; // 18°C
            weather.humidity = 6000; // 60%
            weather.wind_speed = DEFAULT_WIND_SPEED; // 5 m/s
            weather.wind_direction = 180u;
            weather.visibility = select(1000u, 700u, variant == 1u);
            weather.precipitation_rate = 0u;
        } else {
            // Rain
            let intensity = min(variant - 1u, 2u); // Light to Heavy
            weather.weather_type_intensity = WEATHER_RAIN | (intensity << 8u);
            weather.temperature = 150; // 15°C
            weather.humidity = 9000; // 90%
            weather.wind_speed = 150; // 15 m/s
            weather.wind_direction = 180u;
            weather.visibility = 900u - (intensity * 200u);
            weather.precipitation_rate = 100u + (intensity * 200u);
        }
    } else {
        // Default plains weather
        weather.weather_type_intensity = WEATHER_CLEAR;
        weather.temperature = 200; // 20°C
        weather.humidity = MAX_HUMIDITY; // 50%
        weather.wind_speed = DEFAULT_WIND_SPEED; // 5 m/s
        weather.wind_direction = 0u;
        weather.visibility = 1000u;
        weather.precipitation_rate = 0u;
    }
    
    return weather;
}

// Interpolate weather data
fn interpolate_weather(current: WeatherData, target_weather: WeatherData, t: f32) -> WeatherData {
    var result: WeatherData;
    
    // Discrete values use target_weather when t > 0.5
    if (t < 0.5) {
        result.weather_type_intensity = current.weather_type_intensity;
    } else {
        result.weather_type_intensity = target_weather.weather_type_intensity;
    }
    
    // Interpolate continuous values
    result.temperature = i32(mix(f32(current.temperature), f32(target_weather.temperature), t));
    result.humidity = u32(mix(f32(current.humidity), f32(target_weather.humidity), t));
    result.wind_speed = u32(mix(f32(current.wind_speed), f32(target_weather.wind_speed), t));
    
    // Angle interpolation for wind direction
    let current_angle = f32(current.wind_direction) * 3.14159 / 180.0;
    let target_angle = f32(target_weather.wind_direction) * 3.14159 / 180.0;
    let x = mix(cos(current_angle), cos(target_angle), t);
    let y = mix(sin(current_angle), sin(target_angle), t);
    result.wind_direction = u32((atan2(y, x) * 180.0 / 3.14159 + 360.0) % 360.0);
    
    result.visibility = u32(mix(f32(current.visibility), f32(target_weather.visibility), t));
    result.precipitation_rate = u32(mix(f32(current.precipitation_rate), f32(target_weather.precipitation_rate), t));
    
    return result;
}

// Update precipitation particle
fn update_particle(particle: ptr<storage, read_write, PrecipitationParticle>, weather: WeatherData, dt: f32) {
    // Apply gravity based on particle type
    var gravity: f32;
    var wind_effect: f32;
    
    switch ((*particle).particle_type) {
        case 0u: { // Rain
            gravity = -20.0;
            wind_effect = 0.5;
        }
        case 1u: { // Snow
            gravity = -2.0;
            wind_effect = 1.0;
        }
        case 2u: { // Sleet
            gravity = -15.0;
            wind_effect = 0.3;
        }
        default: { // Hail
            gravity = -25.0;
            wind_effect = 0.2;
        }
    }
    
    // Update velocity with gravity and wind
    (*particle).velocity.y += gravity * dt;
    
    // Apply wind
    let wind_angle = f32(weather.wind_direction) * 3.14159 / 180.0;
    let wind_strength = f32(weather.wind_speed) * 0.1;
    (*particle).velocity.x += cos(wind_angle) * wind_strength * wind_effect * dt;
    (*particle).velocity.z += sin(wind_angle) * wind_strength * wind_effect * dt;
    
    // Update position
    (*particle).position += (*particle).velocity * dt;
    
    // Update lifetime
    (*particle).lifetime -= dt * 0.2; // 5 second lifetime
}

// Main compute shader entry point
@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, 
        @builtin(local_invocation_id) local_id: vec3<u32>) {
    
    let region_idx = global_id.x;
    let dt = f32(config.delta_time_ms) * 0.001;
    
    // Initialize workgroup shared memory
    if (local_id.x == 0u) {
        atomicStore(&spawn_count, 0u);
        atomicStore(&next_particle_idx, config.particle_count);
    }
    workgroupBarrier();
    
    // Update weather transitions
    if (region_idx < arrayLength(&transitions)) {
        var transition = transitions[region_idx];
        
        // Update transition progress
        if (transition.progress < 65535u) {
            transition.progress = min(transition.progress + transition.speed, 65535u);
            
            // Interpolate weather
            let t = f32(transition.progress) / 65535.0;
            weather_data[region_idx] = interpolate_weather(transition.current, transition.target_weather, t);
        }
        
        // Check for weather change
        if (transition.change_timer > 0u) {
            transition.change_timer -= 1u;
        } else {
            // Generate new weather based on biome
            let seed = config.random_seed + region_idx * 7919u + config.frame_number;
            let variant = u32(random(seed) * 5.0);
            
            transition.current = weather_data[region_idx];
            transition.target_weather = get_biome_weather(transition.biome_type, variant);
            transition.progress = 0u;
            transition.speed = 10u; // Transition over ~6.5 seconds
            
            // Schedule next change (3-10 minutes at 60 FPS)
            transition.change_timer = 10800u + u32(random(seed + 1337u) * 25200.0);
        }
        
        transitions[region_idx] = transition;
    }
    
    // Update precipitation particles
    if (region_idx < config.particle_count && region_idx < config.max_particles) {
        var particle = particles[region_idx];
        let weather = weather_data[0]; // Use first region's weather for now
        
        update_particle(&particles[region_idx], weather, dt);
        
        // Respawn dead particles
        if (particle.lifetime <= 0.0 || particle.position.y < config.player_position.y - 50.0) {
            // Count particles to spawn
            atomicAdd(&spawn_count, 1u);
        }
    }
    
    workgroupBarrier();
    
    // Spawn new particles (only first few threads)
    if (local_id.x < atomicLoad(&spawn_count) && region_idx < 64u) {
        let new_idx = atomicAdd(&next_particle_idx, 1u);
        if (new_idx < config.max_particles) {
            let weather = weather_data[0];
            let weather_type = weather.weather_type_intensity & 0xFFu;
            
            // Only spawn for precipitating weather
            if (weather_type == WEATHER_RAIN || weather_type == WEATHER_SNOW || weather_type == WEATHER_THUNDERSTORM) {
                var particle: PrecipitationParticle;
                
                // Random spawn position around player
                let seed = config.random_seed + new_idx * 1009u + config.frame_number;
                let angle = random(seed) * 6.28318;
                let distance = random(seed + 1u) * config.precipitation_radius;
                
                particle.position = config.player_position + vec3<f32>(
                    cos(angle) * distance,
                    50.0, // Spawn high above
                    sin(angle) * distance
                );
                
                // Set particle type based on weather
                if (weather_type == WEATHER_SNOW) {
                    particle.particle_type = 1u; // Snow
                    particle.velocity = vec3<f32>(0.0, -2.0, 0.0);
                    particle.size = 0.1;
                } else {
                    particle.particle_type = 0u; // Rain
                    particle.velocity = vec3<f32>(0.0, -10.0, 0.0);
                    particle.size = 0.05;
                }
                
                particle.lifetime = 1.0;
                particles[new_idx] = particle;
            }
        }
    }
}