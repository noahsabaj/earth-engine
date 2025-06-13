use std::io::{Read, Write};
use flate2::Compression as FlateCompression;
use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};

use crate::persistence::{PersistenceResult, PersistenceError};

/// Compression algorithms supported
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionType {
    /// No compression
    None,
    /// Gzip compression (good compression, moderate speed)
    Gzip,
    /// Zlib compression (faster than gzip)
    Zlib,
    /// Zstandard compression (best compression ratio)
    Zstd,
    /// LZ4 compression (fastest)
    Lz4,
}

/// Compression level
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionLevel {
    Fast,
    Default,
    Best,
}

impl CompressionLevel {
    fn to_flate2(&self) -> FlateCompression {
        match self {
            CompressionLevel::Fast => FlateCompression::fast(),
            CompressionLevel::Default => FlateCompression::default(),
            CompressionLevel::Best => FlateCompression::best(),
        }
    }
    
    fn to_zstd(&self) -> i32 {
        match self {
            CompressionLevel::Fast => 1,
            CompressionLevel::Default => 3,
            CompressionLevel::Best => 9,
        }
    }
}

/// Handles compression and decompression of data
pub struct Compressor {
    compression_type: CompressionType,
    compression_level: CompressionLevel,
}

impl Compressor {
    /// Create a new compressor
    pub fn new(compression_type: CompressionType, compression_level: CompressionLevel) -> Self {
        Self {
            compression_type,
            compression_level,
        }
    }
    
    /// Compress data
    pub fn compress(&self, data: &[u8]) -> PersistenceResult<Vec<u8>> {
        match self.compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => self.compress_gzip(data),
            CompressionType::Zlib => self.compress_zlib(data),
            CompressionType::Zstd => self.compress_zstd(data),
            CompressionType::Lz4 => self.compress_lz4(data),
        }
    }
    
    /// Decompress data
    pub fn decompress(&self, data: &[u8]) -> PersistenceResult<Vec<u8>> {
        match self.compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Gzip => self.decompress_gzip(data),
            CompressionType::Zlib => self.decompress_zlib(data),
            CompressionType::Zstd => self.decompress_zstd(data),
            CompressionType::Lz4 => self.decompress_lz4(data),
        }
    }
    
    /// Compress with gzip
    fn compress_gzip(&self, data: &[u8]) -> PersistenceResult<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), self.compression_level.to_flate2());
        encoder.write_all(data)
            .map_err(|e| PersistenceError::CompressionError(format!("Gzip compression failed: {}", e)))?;
        encoder.finish()
            .map_err(|e| PersistenceError::CompressionError(format!("Gzip finalization failed: {}", e)))
    }
    
    /// Decompress with gzip
    fn decompress_gzip(&self, data: &[u8]) -> PersistenceResult<Vec<u8>> {
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| PersistenceError::CompressionError(format!("Gzip decompression failed: {}", e)))?;
        Ok(decompressed)
    }
    
    /// Compress with zlib
    fn compress_zlib(&self, data: &[u8]) -> PersistenceResult<Vec<u8>> {
        let mut encoder = ZlibEncoder::new(Vec::new(), self.compression_level.to_flate2());
        encoder.write_all(data)
            .map_err(|e| PersistenceError::CompressionError(format!("Zlib compression failed: {}", e)))?;
        encoder.finish()
            .map_err(|e| PersistenceError::CompressionError(format!("Zlib finalization failed: {}", e)))
    }
    
    /// Decompress with zlib
    fn decompress_zlib(&self, data: &[u8]) -> PersistenceResult<Vec<u8>> {
        let mut decoder = ZlibDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| PersistenceError::CompressionError(format!("Zlib decompression failed: {}", e)))?;
        Ok(decompressed)
    }
    
    /// Compress with zstd
    fn compress_zstd(&self, data: &[u8]) -> PersistenceResult<Vec<u8>> {
        zstd::encode_all(data, self.compression_level.to_zstd())
            .map_err(|e| PersistenceError::CompressionError(format!("Zstd compression failed: {}", e)))
    }
    
    /// Decompress with zstd
    fn decompress_zstd(&self, data: &[u8]) -> PersistenceResult<Vec<u8>> {
        zstd::decode_all(data)
            .map_err(|e| PersistenceError::CompressionError(format!("Zstd decompression failed: {}", e)))
    }
    
    /// Compress with lz4
    fn compress_lz4(&self, data: &[u8]) -> PersistenceResult<Vec<u8>> {
        Ok(lz4_flex::compress_prepend_size(data))
    }
    
    /// Decompress with lz4
    fn decompress_lz4(&self, data: &[u8]) -> PersistenceResult<Vec<u8>> {
        lz4_flex::decompress_size_prepended(data)
            .map_err(|e| PersistenceError::CompressionError(format!("LZ4 decompression failed: {}", e)))
    }
    
    /// Get compression ratio estimate
    pub fn estimate_ratio(&self, original_size: usize) -> f32 {
        match self.compression_type {
            CompressionType::None => 1.0,
            CompressionType::Gzip => 0.3, // ~70% reduction
            CompressionType::Zlib => 0.35, // ~65% reduction
            CompressionType::Zstd => 0.25, // ~75% reduction
            CompressionType::Lz4 => 0.5, // ~50% reduction
        }
    }
    
    /// Choose best compression for data
    pub fn analyze_data(data: &[u8]) -> (CompressionType, CompressionLevel) {
        let size = data.len();
        
        // Small data: use fast compression
        if size < 1024 {
            return (CompressionType::Lz4, CompressionLevel::Fast);
        }
        
        // Analyze entropy (simple version)
        let mut byte_counts = [0u32; 256];
        for &byte in data {
            byte_counts[byte as usize] += 1;
        }
        
        let mut entropy = 0.0f32;
        for count in byte_counts.iter() {
            if *count > 0 {
                let p = *count as f32 / size as f32;
                entropy -= p * p.log2();
            }
        }
        
        // High entropy (random data): use fast compression
        if entropy > 7.0 {
            (CompressionType::Lz4, CompressionLevel::Fast)
        }
        // Medium entropy: use balanced compression
        else if entropy > 5.0 {
            (CompressionType::Zlib, CompressionLevel::Default)
        }
        // Low entropy (repetitive data): use best compression
        else {
            (CompressionType::Zstd, CompressionLevel::Best)
        }
    }
}

/// Compressed data wrapper with metadata
#[derive(Debug)]
pub struct CompressedData {
    /// Original (uncompressed) size
    pub original_size: usize,
    /// Compressed size
    pub compressed_size: usize,
    /// Compression type used
    pub compression_type: CompressionType,
    /// Compressed data
    pub data: Vec<u8>,
}

impl CompressedData {
    /// Create from uncompressed data
    pub fn compress(data: &[u8], compressor: &Compressor) -> PersistenceResult<Self> {
        let original_size = data.len();
        let compressed = compressor.compress(data)?;
        let compressed_size = compressed.len();
        
        Ok(Self {
            original_size,
            compressed_size,
            compression_type: compressor.compression_type,
            data: compressed,
        })
    }
    
    /// Decompress the data
    pub fn decompress(&self) -> PersistenceResult<Vec<u8>> {
        let compressor = Compressor::new(self.compression_type, CompressionLevel::Default);
        compressor.decompress(&self.data)
    }
    
    /// Get compression ratio
    pub fn compression_ratio(&self) -> f32 {
        if self.original_size == 0 {
            1.0
        } else {
            self.compressed_size as f32 / self.original_size as f32
        }
    }
    
    /// Check if compression was beneficial
    pub fn is_beneficial(&self) -> bool {
        self.compressed_size < self.original_size
    }
}

/// Benchmark different compression algorithms
pub fn benchmark_compression(data: &[u8]) -> Vec<CompressionBenchmark> {
    let mut results = Vec::new();
    
    let types = [
        CompressionType::None,
        CompressionType::Gzip,
        CompressionType::Zlib,
        CompressionType::Zstd,
        CompressionType::Lz4,
    ];
    
    let levels = [
        CompressionLevel::Fast,
        CompressionLevel::Default,
        CompressionLevel::Best,
    ];
    
    for &compression_type in &types {
        for &level in &levels {
            if compression_type == CompressionType::None && level != CompressionLevel::Default {
                continue; // No levels for uncompressed
            }
            
            let compressor = Compressor::new(compression_type, level);
            let start = std::time::Instant::now();
            
            if let Ok(compressed) = compressor.compress(data) {
                let compress_time = start.elapsed();
                
                let decompress_start = std::time::Instant::now();
                if let Ok(_) = compressor.decompress(&compressed) {
                    let decompress_time = decompress_start.elapsed();
                    
                    results.push(CompressionBenchmark {
                        compression_type,
                        level,
                        original_size: data.len(),
                        compressed_size: compressed.len(),
                        compress_time,
                        decompress_time,
                    });
                }
            }
        }
    }
    
    results
}

/// Result of compression benchmark
#[derive(Debug)]
pub struct CompressionBenchmark {
    pub compression_type: CompressionType,
    pub level: CompressionLevel,
    pub original_size: usize,
    pub compressed_size: usize,
    pub compress_time: std::time::Duration,
    pub decompress_time: std::time::Duration,
}

impl CompressionBenchmark {
    pub fn compression_ratio(&self) -> f32 {
        self.compressed_size as f32 / self.original_size as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_compression_types() {
        let data = b"Hello, World! This is a test of compression. This is a test of compression.";
        
        let types = [
            CompressionType::Gzip,
            CompressionType::Zlib,
            CompressionType::Zstd,
            CompressionType::Lz4,
        ];
        
        for &compression_type in &types {
            let compressor = Compressor::new(compression_type, CompressionLevel::Default);
            
            let compressed = compressor.compress(data).expect("Compression should succeed");
            let decompressed = compressor.decompress(&compressed).expect("Decompression should succeed");
            
            assert_eq!(data.to_vec(), decompressed);
            assert!(compressed.len() < data.len()); // Should compress
        }
    }
    
    #[test]
    fn test_compression_levels() {
        let data = vec![42u8; 1024]; // Repetitive data
        
        let levels = [
            CompressionLevel::Fast,
            CompressionLevel::Default,
            CompressionLevel::Best,
        ];
        
        let mut sizes = Vec::new();
        
        for &level in &levels {
            let compressor = Compressor::new(CompressionType::Zstd, level);
            let compressed = compressor.compress(&data).expect("Compression should succeed");
            sizes.push(compressed.len());
        }
        
        // Better compression should produce smaller sizes
        assert!(sizes[0] >= sizes[1]); // Fast >= Default
        assert!(sizes[1] >= sizes[2]); // Default >= Best
    }
    
    #[test]
    fn test_compressed_data() {
        // Use repetitive data that compresses well
        let data = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\
                     bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\
                     cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
        let compressor = Compressor::new(CompressionType::Gzip, CompressionLevel::Default);
        
        let compressed = CompressedData::compress(data, &compressor).expect("Compression should succeed");
        assert!(compressed.is_beneficial());
        assert!(compressed.compression_ratio() < 1.0);
        
        let decompressed = compressed.decompress().expect("Decompression should succeed");
        assert_eq!(data.to_vec(), decompressed);
    }
}