/// Simple test to verify Morton encoding is working

fn main() {
    println!("Morton Encoding Test");
    println!("====================\n");
    
    // Basic Morton encoding test
    test_morton_encoding();
    
    // Performance test
    test_morton_performance();
}

/// Magic numbers for bit spreading/compacting
const MAGIC_X: u64 = 0x1249249249249249;
const MAGIC_Y: u64 = 0x2492492492492492;
const MAGIC_Z: u64 = 0x4924924924924924;

/// Spreads bits of a 21-bit integer to every 3rd bit
#[inline(always)]
fn spread_bits(mut v: u32) -> u64 {
    v = (v | (v << 16)) & 0x030000FF;
    v = (v | (v << 8)) & 0x0300F00F;
    v = (v | (v << 4)) & 0x030C30C3;
    v = (v | (v << 2)) & 0x09249249;
    v as u64
}

/// Compacts every 3rd bit back to a 21-bit integer
#[inline(always)]
fn compact_bits(mut v: u64) -> u32 {
    v &= 0x09249249;
    v = (v | (v >> 2)) & 0x030C30C3;
    v = (v | (v >> 4)) & 0x0300F00F;
    v = (v | (v >> 8)) & 0x030000FF;
    v = (v | (v >> 16)) & 0x0000FFFF;
    v as u32
}

/// Encode 3D coordinates to Morton code
#[inline(always)]
pub fn morton_encode(x: u32, y: u32, z: u32) -> u64 {
    spread_bits(x) | (spread_bits(y) << 1) | (spread_bits(z) << 2)
}

/// Decode Morton code to 3D coordinates
#[inline(always)]
pub fn morton_decode(morton: u64) -> (u32, u32, u32) {
    let x = compact_bits(morton);
    let y = compact_bits(morton >> 1);
    let z = compact_bits(morton >> 2);
    (x, y, z)
}

fn test_morton_encoding() {
    println!("Testing Morton encoding/decoding...");
    
    let test_cases = vec![
        (0, 0, 0),
        (1, 1, 1),
        (7, 7, 7),
        (15, 15, 15),
        (31, 31, 31),
        (100, 200, 50),
        (1000, 500, 750),
    ];
    
    for (x, y, z) in test_cases {
        let morton = morton_encode(x, y, z);
        let (dx, dy, dz) = morton_decode(morton);
        
        println!("({}, {}, {}) -> {} -> ({}, {}, {})", x, y, z, morton, dx, dy, dz);
        
        assert_eq!((x, y, z), (dx, dy, dz), "Encoding/decoding mismatch!");
    }
    
    println!("✓ All encoding/decoding tests passed!\n");
}

fn test_morton_performance() {
    use std::time::Instant;
    
    println!("Testing Morton encoding performance...");
    
    const ITERATIONS: u32 = 10_000_000;
    
    // Test encoding performance
    let start = Instant::now();
    let mut sum = 0u64;
    for i in 0..ITERATIONS {
        let x = i % 1024;
        let y = (i / 1024) % 1024;
        let z = (i / (1024 * 1024)) % 1024;
        sum = sum.wrapping_add(morton_encode(x, y, z));
    }
    let encode_time = start.elapsed();
    
    // Test decoding performance
    let start = Instant::now();
    let mut sum2 = 0u32;
    for i in 0..ITERATIONS {
        let (x, y, z) = morton_decode(i as u64);
        sum2 = sum2.wrapping_add(x).wrapping_add(y).wrapping_add(z);
    }
    let decode_time = start.elapsed();
    
    println!("Encoding {} coordinates: {:?}", ITERATIONS, encode_time);
    println!("  Rate: {:.2} million coords/sec", ITERATIONS as f64 / encode_time.as_secs_f64() / 1_000_000.0);
    
    println!("Decoding {} coordinates: {:?}", ITERATIONS, decode_time);
    println!("  Rate: {:.2} million coords/sec", ITERATIONS as f64 / decode_time.as_secs_f64() / 1_000_000.0);
    
    // Prevent optimization
    println!("\n(Checksums: {} {})", sum, sum2);
}