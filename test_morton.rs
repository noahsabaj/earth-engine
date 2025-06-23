fn morton_encode_chunk(x: u32, y: u32, z: u32) -> u32 {
    let mut result = 0u32;
    for i in 0..6 {
        result  < /dev/null | = ((x >> i) & 1) << (i * 3);
        result |= ((y >> i) & 1) << (i * 3 + 1);
        result |= ((z >> i) & 1) << (i * 3 + 2);
    }
    result
}

fn main() {
    let morton_49 = morton_encode_chunk(49, 49, 49);
    println\!("Morton index for (49,49,49): {}", morton_49);
    println\!("That's {} times larger than needed for 125k voxels\!", morton_49 / 125000);
    
    // Binary representation of 49: 110001
    println\!("49 in binary: {:06b}", 49);
    println\!("Morton result in binary: {:018b}", morton_49);
}
