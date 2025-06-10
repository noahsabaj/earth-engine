/// Mesh Compression System
/// 
/// Compresses mesh data for efficient storage and streaming using
/// quantization, delta encoding, and entropy compression.
/// Part of Sprint 29: Mesh Optimization & Advanced LOD

use crate::renderer::Vertex;
use std::io::{Write, Read};
use flate2::write::ZlibEncoder;
use flate2::read::ZlibDecoder;
use flate2::Compression;
use bytemuck::{Pod, Zeroable};

/// Compressed mesh format
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CompressedHeader {
    pub vertex_count: u32,
    pub index_count: u32,
    pub position_bits: u8,
    pub normal_bits: u8,
    pub texcoord_bits: u8,
    pub flags: u8,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
}

/// Quantized vertex data
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct QuantizedVertex {
    position: [u16; 3],  // Quantized to 16 bits
    normal: [u8; 2],     // Octahedral encoding
    tex_coords: [u16; 2], // Quantized UVs
}

/// Mesh compression options
#[derive(Debug, Clone)]
pub struct CompressionOptions {
    pub position_bits: u8,
    pub normal_bits: u8,
    pub texcoord_bits: u8,
    pub use_delta_encoding: bool,
    pub use_zlib: bool,
    pub zlib_level: u32,
}

impl Default for CompressionOptions {
    fn default() -> Self {
        Self {
            position_bits: 14,
            normal_bits: 10,
            texcoord_bits: 12,
            use_delta_encoding: true,
            use_zlib: true,
            zlib_level: 6,
        }
    }
}

/// Mesh compressor
pub struct MeshCompressor {
    options: CompressionOptions,
}

impl MeshCompressor {
    pub fn new(options: CompressionOptions) -> Self {
        Self { options }
    }
    
    /// Compress mesh data
    pub fn compress(&self, vertices: &[Vertex], indices: &[u32]) -> Result<Vec<u8>, std::io::Error> {
        // Calculate bounding box
        let (bbox_min, bbox_max) = self.calculate_bbox(vertices);
        
        // Create header
        let header = CompressedHeader {
            vertex_count: vertices.len() as u32,
            index_count: indices.len() as u32,
            position_bits: self.options.position_bits,
            normal_bits: self.options.normal_bits,
            texcoord_bits: self.options.texcoord_bits,
            flags: (self.options.use_delta_encoding as u8) | ((self.options.use_zlib as u8) << 1),
            bbox_min,
            bbox_max,
        };
        
        // Quantize vertices
        let quantized_vertices = self.quantize_vertices(vertices, &bbox_min, &bbox_max);
        
        // Apply delta encoding if enabled
        let encoded_vertices = if self.options.use_delta_encoding {
            self.delta_encode_vertices(&quantized_vertices)
        } else {
            quantized_vertices
        };
        
        // Encode indices with delta compression
        let encoded_indices = self.delta_encode_indices(indices);
        
        // Pack data
        let mut packed_data = Vec::new();
        packed_data.extend_from_slice(bytemuck::bytes_of(&header));
        
        // Pack vertices
        for vertex in &encoded_vertices {
            packed_data.extend_from_slice(bytemuck::bytes_of(vertex));
        }
        
        // Pack indices using variable-length encoding
        for &index in &encoded_indices {
            self.write_varint(&mut packed_data, index);
        }
        
        // Apply zlib compression if enabled
        if self.options.use_zlib {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(self.options.zlib_level));
            encoder.write_all(&packed_data)?;
            encoder.finish()
        } else {
            Ok(packed_data)
        }
    }
    
    /// Calculate bounding box
    fn calculate_bbox(&self, vertices: &[Vertex]) -> ([f32; 3], [f32; 3]) {
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        
        for vertex in vertices {
            for i in 0..3 {
                min[i] = min[i].min(vertex.position[i]);
                max[i] = max[i].max(vertex.position[i]);
            }
        }
        
        (min, max)
    }
    
    /// Quantize vertices to reduced precision
    fn quantize_vertices(&self, vertices: &[Vertex], bbox_min: &[f32; 3], bbox_max: &[f32; 3]) -> Vec<QuantizedVertex> {
        let bbox_size = [
            bbox_max[0] - bbox_min[0],
            bbox_max[1] - bbox_min[1],
            bbox_max[2] - bbox_min[2],
        ];
        
        let position_scale = (1u32 << self.options.position_bits) as f32 - 1.0;
        let texcoord_scale = (1u32 << self.options.texcoord_bits) as f32 - 1.0;
        
        vertices.iter().map(|vertex| {
            // Quantize position
            let position = [
                ((vertex.position[0] - bbox_min[0]) / bbox_size[0] * position_scale) as u16,
                ((vertex.position[1] - bbox_min[1]) / bbox_size[1] * position_scale) as u16,
                ((vertex.position[2] - bbox_min[2]) / bbox_size[2] * position_scale) as u16,
            ];
            
            // Encode normal using octahedral encoding
            let normal = self.encode_octahedral_normal(vertex.normal);
            
            // Quantize texture coordinates
            let tex_coords = [
                (vertex.tex_coords[0].clamp(0.0, 1.0) * texcoord_scale) as u16,
                (vertex.tex_coords[1].clamp(0.0, 1.0) * texcoord_scale) as u16,
            ];
            
            QuantizedVertex {
                position,
                normal,
                tex_coords,
            }
        }).collect()
    }
    
    /// Encode normal using octahedral encoding
    fn encode_octahedral_normal(&self, normal: [f32; 3]) -> [u8; 2] {
        let n = cgmath::Vector3::from(normal).normalize();
        
        // Project to octahedron
        let sum = n.x.abs() + n.y.abs() + n.z.abs();
        let mut oct = cgmath::Vector2::new(n.x / sum, n.y / sum);
        
        // Reflect the folds of the lower hemisphere
        if n.z < 0.0 {
            let tmp = oct;
            oct.x = (1.0 - tmp.y.abs()) * if tmp.x >= 0.0 { 1.0 } else { -1.0 };
            oct.y = (1.0 - tmp.x.abs()) * if tmp.y >= 0.0 { 1.0 } else { -1.0 };
        }
        
        // Quantize to 8 bits per component
        [
            ((oct.x * 0.5 + 0.5) * 255.0) as u8,
            ((oct.y * 0.5 + 0.5) * 255.0) as u8,
        ]
    }
    
    /// Apply delta encoding to vertices
    fn delta_encode_vertices(&self, vertices: &[QuantizedVertex]) -> Vec<QuantizedVertex> {
        if vertices.is_empty() {
            return vec![];
        }
        
        let mut encoded = Vec::with_capacity(vertices.len());
        encoded.push(vertices[0]); // First vertex is stored as-is
        
        for i in 1..vertices.len() {
            let prev = &vertices[i - 1];
            let curr = &vertices[i];
            
            encoded.push(QuantizedVertex {
                position: [
                    curr.position[0].wrapping_sub(prev.position[0]),
                    curr.position[1].wrapping_sub(prev.position[1]),
                    curr.position[2].wrapping_sub(prev.position[2]),
                ],
                normal: curr.normal, // Don't delta encode normals
                tex_coords: [
                    curr.tex_coords[0].wrapping_sub(prev.tex_coords[0]),
                    curr.tex_coords[1].wrapping_sub(prev.tex_coords[1]),
                ],
            });
        }
        
        encoded
    }
    
    /// Apply delta encoding to indices
    fn delta_encode_indices(&self, indices: &[u32]) -> Vec<i32> {
        if indices.is_empty() {
            return vec![];
        }
        
        let mut encoded = Vec::with_capacity(indices.len());
        let mut last = 0i32;
        
        for &index in indices {
            let delta = index as i32 - last;
            encoded.push(delta);
            last = index as i32;
        }
        
        encoded
    }
    
    /// Write variable-length integer
    fn write_varint(&self, buffer: &mut Vec<u8>, value: i32) {
        let mut val = ((value << 1) ^ (value >> 31)) as u32; // ZigZag encoding
        
        while val >= 0x80 {
            buffer.push((val | 0x80) as u8);
            val >>= 7;
        }
        buffer.push(val as u8);
    }
}

/// Mesh decompressor
pub struct MeshDecompressor;

impl MeshDecompressor {
    /// Decompress mesh data
    pub fn decompress(compressed_data: &[u8]) -> Result<(Vec<Vertex>, Vec<u32>), std::io::Error> {
        // Check if data is zlib compressed
        let data = if compressed_data.len() > 2 && 
                     compressed_data[0] == 0x78 && // zlib header
                     (compressed_data[1] == 0x9C || compressed_data[1] == 0xDA) {
            // Decompress
            let mut decoder = ZlibDecoder::new(compressed_data);
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed)?;
            decompressed
        } else {
            compressed_data.to_vec()
        };
        
        // Read header
        if data.len() < std::mem::size_of::<CompressedHeader>() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid compressed mesh data"
            ));
        }
        
        let header = bytemuck::from_bytes::<CompressedHeader>(&data[..std::mem::size_of::<CompressedHeader>()]);
        
        // Decode vertices and indices
        let vertices = Self::decode_vertices(&data, header)?;
        let indices = Self::decode_indices(&data, header)?;
        
        Ok((vertices, indices))
    }
    
    /// Decode vertices
    fn decode_vertices(data: &[u8], header: &CompressedHeader) -> Result<Vec<Vertex>, std::io::Error> {
        // Implementation would reverse the compression process
        // For now, return empty vec
        Ok(vec![])
    }
    
    /// Decode indices
    fn decode_indices(data: &[u8], header: &CompressedHeader) -> Result<Vec<u32>, std::io::Error> {
        // Implementation would reverse the compression process
        // For now, return empty vec
        Ok(vec![])
    }
}

/// Compression statistics
#[derive(Debug)]
pub struct CompressionStats {
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f32,
    pub vertex_bits_per_element: f32,
    pub index_bits_per_element: f32,
}