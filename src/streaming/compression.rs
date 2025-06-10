use wgpu::{Device, ComputePipeline, BindGroup, Buffer};
use bytemuck::{Pod, Zeroable};

/// Compression types supported by GPU
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    None = 0,
    RLE = 1,          // Run-length encoding
    BitPacked = 2,    // Bit-packed sparse voxels
    Palettized = 3,   // Palette compression
    Hybrid = 4,       // Combined techniques
}

/// Compression header for GPU
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CompressionHeader {
    pub compression_type: u8,
    pub uncompressed_size: u32,
    pub compressed_size: u32,
    pub block_count: u32,
    pub palette_size: u16,
    pub _padding: u16,
}

/// GPU decompressor
pub struct GpuDecompressor {
    /// Decompression pipelines
    rle_pipeline: ComputePipeline,
    bitpacked_pipeline: ComputePipeline,
    palettized_pipeline: ComputePipeline,
    hybrid_pipeline: ComputePipeline,
    
    /// Scratch buffers for decompression
    scratch_buffers: Vec<Buffer>,
    
    /// Device reference
    device: wgpu::Device,
}

impl GpuDecompressor {
    /// Create new GPU decompressor
    pub fn new(device: &Device) -> Self {
        // Create decompression pipelines
        let rle_pipeline = create_rle_pipeline(device);
        let bitpacked_pipeline = create_bitpacked_pipeline(device);
        let palettized_pipeline = create_palettized_pipeline(device);
        let hybrid_pipeline = create_hybrid_pipeline(device);
        
        // Pre-allocate scratch buffers
        let mut scratch_buffers = Vec::new();
        for i in 0..4 {
            scratch_buffers.push(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("Decompression Scratch {}", i)),
                size: 1024 * 1024, // 1MB scratch space
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }));
        }
        
        Self {
            rle_pipeline,
            bitpacked_pipeline,
            palettized_pipeline,
            hybrid_pipeline,
            scratch_buffers,
            device: device.clone(),
        }
    }
    
    /// Get appropriate pipeline for compression type
    pub fn get_pipeline(&self, compression_type: CompressionType) -> &ComputePipeline {
        match compression_type {
            CompressionType::None => unreachable!("No pipeline for uncompressed data"),
            CompressionType::RLE => &self.rle_pipeline,
            CompressionType::BitPacked => &self.bitpacked_pipeline,
            CompressionType::Palettized => &self.palettized_pipeline,
            CompressionType::Hybrid => &self.hybrid_pipeline,
        }
    }
    
    /// Get scratch buffer for decompression
    pub fn get_scratch_buffer(&mut self, index: usize) -> &Buffer {
        &self.scratch_buffers[index % self.scratch_buffers.len()]
    }
}

/// Create RLE decompression pipeline
fn create_rle_pipeline(device: &Device) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("RLE Decompression Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/decompress_rle.wgsl").into()),
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("RLE Decompression Pipeline"),
        layout: None,
        module: &shader,
        entry_point: "decompress_rle",
    })
}

/// Create bit-packed decompression pipeline
fn create_bitpacked_pipeline(device: &Device) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("BitPacked Decompression Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/decompress_bitpacked.wgsl").into()),
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("BitPacked Decompression Pipeline"),
        layout: None,
        module: &shader,
        entry_point: "decompress_bitpacked",
    })
}

/// Create palettized decompression pipeline
fn create_palettized_pipeline(device: &Device) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Palettized Decompression Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/decompress_palettized.wgsl").into()),
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Palettized Decompression Pipeline"),
        layout: None,
        module: &shader,
        entry_point: "decompress_palettized",
    })
}

/// Create hybrid decompression pipeline
fn create_hybrid_pipeline(device: &Device) -> ComputePipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Hybrid Decompression Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/decompress_hybrid.wgsl").into()),
    });
    
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Hybrid Decompression Pipeline"),
        layout: None,
        module: &shader,
        entry_point: "decompress_hybrid",
    })
}

/// CPU-side compression for background processing
pub mod cpu_compressor {
    use super::*;
    use crate::streaming::PAGE_VOXEL_COUNT;
    
    /// Compress voxel data using RLE
    pub fn compress_rle(voxels: &[u32]) -> Vec<u8> {
        let mut compressed = Vec::new();
        let mut i = 0;
        
        while i < voxels.len() {
            let value = voxels[i];
            let mut count = 1;
            
            // Count consecutive identical values
            while i + count < voxels.len() && 
                  voxels[i + count] == value && 
                  count < 255 {
                count += 1;
            }
            
            // Write count and value
            compressed.push(count as u8);
            compressed.extend_from_slice(&value.to_le_bytes());
            
            i += count;
        }
        
        compressed
    }
    
    /// Compress using bit-packing for sparse data
    pub fn compress_bitpacked(voxels: &[u32]) -> Vec<u8> {
        let mut compressed = Vec::new();
        let mut non_empty = Vec::new();
        let mut bitmap = vec![0u8; (voxels.len() + 7) / 8];
        
        // Build bitmap and collect non-empty voxels
        for (i, &voxel) in voxels.iter().enumerate() {
            if voxel != 0 {
                bitmap[i / 8] |= 1 << (i % 8);
                non_empty.push(voxel);
            }
        }
        
        // Write bitmap size and data
        compressed.extend_from_slice(&(bitmap.len() as u32).to_le_bytes());
        compressed.extend_from_slice(&bitmap);
        
        // Write non-empty voxels
        compressed.extend_from_slice(&(non_empty.len() as u32).to_le_bytes());
        for voxel in non_empty {
            compressed.extend_from_slice(&voxel.to_le_bytes());
        }
        
        compressed
    }
    
    /// Compress using palette for limited block types
    pub fn compress_palettized(voxels: &[u32]) -> Vec<u8> {
        use std::collections::HashMap;
        
        let mut compressed = Vec::new();
        let mut palette = Vec::new();
        let mut palette_map = HashMap::new();
        
        // Build palette
        for &voxel in voxels {
            if !palette_map.contains_key(&voxel) {
                let index = palette.len() as u16;
                palette_map.insert(voxel, index);
                palette.push(voxel);
                
                // Limit palette size
                if palette.len() >= 256 {
                    break;
                }
            }
        }
        
        // Write palette
        compressed.extend_from_slice(&(palette.len() as u16).to_le_bytes());
        for &value in &palette {
            compressed.extend_from_slice(&value.to_le_bytes());
        }
        
        // Write indices
        if palette.len() <= 16 {
            // 4-bit indices
            for chunk in voxels.chunks(2) {
                let idx0 = palette_map.get(&chunk[0]).copied().unwrap_or(0) as u8;
                let idx1 = if chunk.len() > 1 {
                    palette_map.get(&chunk[1]).copied().unwrap_or(0) as u8
                } else {
                    0
                };
                compressed.push((idx0 << 4) | (idx1 & 0xF));
            }
        } else {
            // 8-bit indices
            for &voxel in voxels {
                let idx = palette_map.get(&voxel).copied().unwrap_or(0) as u8;
                compressed.push(idx);
            }
        }
        
        compressed
    }
    
    /// Choose best compression method
    pub fn compress_auto(voxels: &[u32]) -> (CompressionType, Vec<u8>) {
        // Try different compression methods
        let rle = compress_rle(voxels);
        let bitpacked = compress_bitpacked(voxels);
        let palettized = compress_palettized(voxels);
        
        // Choose smallest
        let mut best_type = CompressionType::None;
        let mut best_size = voxels.len() * 4;
        let mut best_data = Vec::new();
        
        if rle.len() < best_size {
            best_type = CompressionType::RLE;
            best_size = rle.len();
            best_data = rle;
        }
        
        if bitpacked.len() < best_size {
            best_type = CompressionType::BitPacked;
            best_size = bitpacked.len();
            best_data = bitpacked;
        }
        
        if palettized.len() < best_size {
            best_type = CompressionType::Palettized;
            best_data = palettized;
        }
        
        (best_type, best_data)
    }
}

/// Compression worker for background processing
pub struct CompressionWorker {
    /// Compression requests
    requests: flume::Receiver<CompressionRequest>,
    
    /// Compression results
    results: flume::Sender<CompressionResult>,
}

pub struct CompressionRequest {
    pub page_index: usize,
    pub voxels: Vec<u32>,
}

pub struct CompressionResult {
    pub page_index: usize,
    pub compression_type: CompressionType,
    pub compressed_data: Vec<u8>,
    pub compression_ratio: f32,
}

impl CompressionWorker {
    /// Create new compression worker
    pub fn new() -> (Self, flume::Sender<CompressionRequest>, flume::Receiver<CompressionResult>) {
        let (req_tx, req_rx) = flume::unbounded();
        let (res_tx, res_rx) = flume::unbounded();
        
        (Self {
            requests: req_rx,
            results: res_tx,
        }, req_tx, res_rx)
    }
    
    /// Run compression worker
    pub fn run(self) {
        while let Ok(request) = self.requests.recv() {
            let uncompressed_size = request.voxels.len() * 4;
            let (compression_type, compressed_data) = 
                cpu_compressor::compress_auto(&request.voxels);
            
            let compression_ratio = compressed_data.len() as f32 / uncompressed_size as f32;
            
            self.results.send(CompressionResult {
                page_index: request.page_index,
                compression_type,
                compressed_data,
                compression_ratio,
            }).ok();
        }
    }
}