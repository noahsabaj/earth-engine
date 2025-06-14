use serde::{Serialize, Deserialize};
use crate::world::{Chunk, BlockId, VoxelPos, ChunkPos};
use crate::persistence::{PersistenceResult, PersistenceError};

/// Version of the chunk format
const CHUNK_FORMAT_VERSION: u32 = 1;

/// Magic bytes to identify chunk files
const CHUNK_MAGIC: &[u8] = b"ECNK";

/// Chunk serialization format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChunkFormat {
    /// Raw uncompressed format
    Raw,
    /// Run-length encoded format for sparse chunks
    RLE,
    /// Palette-based format for chunks with few block types
    Palette,
}

/// Header for serialized chunks
#[derive(Debug, Serialize, Deserialize)]
struct ChunkHeader {
    magic: [u8; 4],
    version: u32,
    format: u8,
    chunk_pos: ChunkPos,
    block_count: u32,
    timestamp: u64,
    checksum: u32,
}

/// Serializes and deserializes chunks
#[derive(Debug)]
pub struct ChunkSerializer {
    format: ChunkFormat,
}

impl ChunkSerializer {
    pub fn new(format: ChunkFormat) -> Self {
        Self { format }
    }
    
    /// Serialize a chunk to bytes
    pub fn serialize(&self, chunk: &Chunk) -> PersistenceResult<Vec<u8>> {
        match self.format {
            ChunkFormat::Raw => self.serialize_raw(chunk),
            ChunkFormat::RLE => self.serialize_rle(chunk),
            ChunkFormat::Palette => self.serialize_palette(chunk),
        }
    }
    
    /// Deserialize a chunk from bytes
    pub fn deserialize(&self, data: &[u8]) -> PersistenceResult<Chunk> {
        // Validate minimum data size
        if data.len() < 32 {
            return Err(PersistenceError::CorruptedData(
                "Data too small to contain valid chunk header".to_string()
            ));
        }
        
        // Read and validate header
        let header = self.read_header(data)?;
        
        if header.magic != *CHUNK_MAGIC {
            return Err(PersistenceError::CorruptedData("Invalid chunk magic".to_string()));
        }
        
        if header.version != CHUNK_FORMAT_VERSION {
            return Err(PersistenceError::VersionMismatch {
                expected: CHUNK_FORMAT_VERSION,
                found: header.version,
            });
        }
        
        // Validate block count is reasonable (chunks can't have more blocks than their volume)
        const MAX_CHUNK_VOLUME: u32 = 32 * 32 * 32; // Typical chunk size
        if header.block_count > MAX_CHUNK_VOLUME {
            return Err(PersistenceError::CorruptedData(
                format!("Invalid block count: {} exceeds maximum {}", 
                    header.block_count, MAX_CHUNK_VOLUME)
            ));
        }
        
        // Calculate header size
        let header_size = bincode::serialized_size(&header)? as usize;
        
        // Validate data has enough bytes for the content
        if data.len() < header_size {
            return Err(PersistenceError::CorruptedData(
                "Data smaller than header size".to_string()
            ));
        }
        
        // Verify checksum
        let checksum = self.calculate_checksum(&data[header_size..]);
        if checksum != header.checksum {
            return Err(PersistenceError::CorruptedData("Checksum mismatch".to_string()));
        }
        
        // Deserialize based on format
        match header.format {
            0 => self.deserialize_raw(data, header),
            1 => self.deserialize_rle(data, header),
            2 => self.deserialize_palette(data, header),
            _ => Err(PersistenceError::DeserializationError("Unknown chunk format".to_string())),
        }
    }
    
    /// Serialize chunk in raw format (uncompressed)
    fn serialize_raw(&self, chunk: &Chunk) -> PersistenceResult<Vec<u8>> {
        let mut buffer = Vec::new();
        
        // Reserve space for header
        let header_size = bincode::serialized_size(&ChunkHeader {
            magic: [0; 4],
            version: 0,
            format: 0,
            chunk_pos: ChunkPos { x: 0, y: 0, z: 0 },
            block_count: 0,
            timestamp: 0,
            checksum: 0,
        })? as usize;
        buffer.resize(header_size, 0);
        
        // Write block data
        for y in 0..chunk.size() {
            for z in 0..chunk.size() {
                for x in 0..chunk.size() {
                    let pos = VoxelPos::new(x as i32, y as i32, z as i32);
                    let block = chunk.get_block_at(pos);
                    buffer.extend_from_slice(&block.0.to_le_bytes());
                }
            }
        }
        
        // Calculate checksum of data (excluding header)
        let checksum = self.calculate_checksum(&buffer[header_size..]);
        
        // Create header with checksum
        let header = ChunkHeader {
            magic: [CHUNK_MAGIC[0], CHUNK_MAGIC[1], CHUNK_MAGIC[2], CHUNK_MAGIC[3]],
            version: CHUNK_FORMAT_VERSION,
            format: 0, // Raw format
            chunk_pos: chunk.position(),
            block_count: chunk.size() * chunk.size() * chunk.size(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_secs(),
            checksum,
        };
        
        // Write header at the beginning
        let header_bytes = bincode::serialize(&header)?;
        buffer[..header_size].copy_from_slice(&header_bytes);
        
        Ok(buffer)
    }
    
    /// Serialize chunk in RLE format (run-length encoded)
    fn serialize_rle(&self, chunk: &Chunk) -> PersistenceResult<Vec<u8>> {
        let mut buffer = Vec::new();
        
        // Reserve space for header
        let header_size = bincode::serialized_size(&ChunkHeader {
            magic: [0; 4],
            version: 0,
            format: 0,
            chunk_pos: ChunkPos { x: 0, y: 0, z: 0 },
            block_count: 0,
            timestamp: 0,
            checksum: 0,
        })? as usize;
        buffer.resize(header_size, 0);
        
        // RLE encode the blocks
        let mut runs = Vec::new();
        let mut current_block = chunk.get_block_at(VoxelPos::new(0, 0, 0));
        let mut run_length = 1u32;
        
        for y in 0..chunk.size() {
            for z in 0..chunk.size() {
                for x in 0..chunk.size() {
                    if x == 0 && y == 0 && z == 0 { continue; }
                    
                    let pos = VoxelPos::new(x as i32, y as i32, z as i32);
                    let block = chunk.get_block_at(pos);
                    
                    if block == current_block && run_length < u32::MAX {
                        run_length += 1;
                    } else {
                        runs.push((current_block, run_length));
                        current_block = block;
                        run_length = 1;
                    }
                }
            }
        }
        runs.push((current_block, run_length));
        
        // Write run count
        buffer.extend_from_slice(&(runs.len() as u32).to_le_bytes());
        
        // Write runs
        for (block, length) in runs {
            buffer.extend_from_slice(&block.0.to_le_bytes());
            buffer.extend_from_slice(&length.to_le_bytes());
        }
        
        // Calculate checksum of data (excluding header)
        let checksum = self.calculate_checksum(&buffer[header_size..]);
        
        // Create header with checksum
        let header = ChunkHeader {
            magic: [CHUNK_MAGIC[0], CHUNK_MAGIC[1], CHUNK_MAGIC[2], CHUNK_MAGIC[3]],
            version: CHUNK_FORMAT_VERSION,
            format: 1, // RLE format
            chunk_pos: chunk.position(),
            block_count: chunk.size() * chunk.size() * chunk.size(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_secs(),
            checksum,
        };
        
        // Write header at the beginning
        let header_bytes = bincode::serialize(&header)?;
        buffer[..header_size].copy_from_slice(&header_bytes);
        
        Ok(buffer)
    }
    
    /// Serialize chunk in palette format
    fn serialize_palette(&self, chunk: &Chunk) -> PersistenceResult<Vec<u8>> {
        let mut buffer = Vec::new();
        
        // Build palette of unique blocks
        let mut palette = Vec::new();
        let mut block_to_palette = std::collections::HashMap::new();
        
        for y in 0..chunk.size() {
            for z in 0..chunk.size() {
                for x in 0..chunk.size() {
                    let pos = VoxelPos::new(x as i32, y as i32, z as i32);
                    let block = chunk.get_block_at(pos);
                    
                    if !block_to_palette.contains_key(&block) {
                        let index = palette.len() as u8;
                        palette.push(block);
                        block_to_palette.insert(block, index);
                    }
                }
            }
        }
        
        // Use palette format only if it saves space
        if palette.len() > 256 || palette.len() > chunk.size() as usize {
            return self.serialize_raw(chunk);
        }
        
        // Reserve space for header
        let header_size = bincode::serialized_size(&ChunkHeader {
            magic: [0; 4],
            version: 0,
            format: 0,
            chunk_pos: ChunkPos { x: 0, y: 0, z: 0 },
            block_count: 0,
            timestamp: 0,
            checksum: 0,
        })? as usize;
        buffer.resize(header_size, 0);
        
        // Write palette size
        buffer.push(palette.len() as u8);
        
        // Write palette
        for block in &palette {
            buffer.extend_from_slice(&block.0.to_le_bytes());
        }
        
        // Write block indices
        for y in 0..chunk.size() {
            for z in 0..chunk.size() {
                for x in 0..chunk.size() {
                    let pos = VoxelPos::new(x as i32, y as i32, z as i32);
                    let block = chunk.get_block_at(pos);
                    let index = block_to_palette[&block];
                    buffer.push(index);
                }
            }
        }
        
        // Calculate checksum of data (excluding header)
        let checksum = self.calculate_checksum(&buffer[header_size..]);
        
        // Create header with checksum
        let header = ChunkHeader {
            magic: [CHUNK_MAGIC[0], CHUNK_MAGIC[1], CHUNK_MAGIC[2], CHUNK_MAGIC[3]],
            version: CHUNK_FORMAT_VERSION,
            format: 2, // Palette format
            chunk_pos: chunk.position(),
            block_count: chunk.size() * chunk.size() * chunk.size(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_secs(),
            checksum,
        };
        
        // Write header at the beginning
        let header_bytes = bincode::serialize(&header)?;
        buffer[..header_size].copy_from_slice(&header_bytes);
        
        Ok(buffer)
    }
    
    /// Deserialize chunk from raw format
    fn deserialize_raw(&self, data: &[u8], header: ChunkHeader) -> PersistenceResult<Chunk> {
        let mut chunk = Chunk::new(header.chunk_pos, 32); // TODO: get chunk size from header
        let header_size = bincode::serialized_size(&header)? as usize;
        let mut cursor = header_size;
        
        for y in 0..chunk.size() {
            for z in 0..chunk.size() {
                for x in 0..chunk.size() {
                    if cursor + 2 > data.len() {
                        return Err(PersistenceError::CorruptedData("Unexpected end of data".to_string()));
                    }
                    
                    let block_id = u16::from_le_bytes([
                        data[cursor], data[cursor + 1]
                    ]);
                    
                    let pos = VoxelPos::new(x as i32, y as i32, z as i32);
                    chunk.set_block_at(pos, BlockId(block_id));
                    cursor += 2;
                }
            }
        }
        
        Ok(chunk)
    }
    
    /// Deserialize chunk from RLE format
    fn deserialize_rle(&self, data: &[u8], header: ChunkHeader) -> PersistenceResult<Chunk> {
        let mut chunk = Chunk::new(header.chunk_pos, 32);
        let header_size = bincode::serialized_size(&header)? as usize;
        let mut cursor = header_size;
        
        // Read run count
        if cursor + 4 > data.len() {
            return Err(PersistenceError::CorruptedData("Missing run count".to_string()));
        }
        let run_count = u32::from_le_bytes([
            data[cursor], data[cursor + 1], data[cursor + 2], data[cursor + 3]
        ]) as usize;
        cursor += 4;
        
        // Read runs and fill chunk
        let mut block_index = 0;
        for _ in 0..run_count {
            if cursor + 6 > data.len() {
                return Err(PersistenceError::CorruptedData("Incomplete run data".to_string()));
            }
            
            let block_id = u16::from_le_bytes([
                data[cursor], data[cursor + 1]
            ]);
            let run_length = u32::from_le_bytes([
                data[cursor + 2], data[cursor + 3], data[cursor + 4], data[cursor + 5]
            ]);
            cursor += 6;
            
            for _ in 0..run_length {
                if block_index >= header.block_count {
                    return Err(PersistenceError::CorruptedData("Too many blocks".to_string()));
                }
                
                let x = (block_index % chunk.size()) as i32;
                let z = ((block_index / chunk.size()) % chunk.size()) as i32;
                let y = (block_index / (chunk.size() * chunk.size())) as i32;
                
                chunk.set_block_at(VoxelPos::new(x, y, z), BlockId(block_id));
                block_index += 1;
            }
        }
        
        Ok(chunk)
    }
    
    /// Deserialize chunk from palette format
    fn deserialize_palette(&self, data: &[u8], header: ChunkHeader) -> PersistenceResult<Chunk> {
        let mut chunk = Chunk::new(header.chunk_pos, 32);
        let header_size = bincode::serialized_size(&header)? as usize;
        let mut cursor = header_size;
        
        // Read palette size
        if cursor >= data.len() {
            return Err(PersistenceError::CorruptedData("Missing palette size".to_string()));
        }
        let palette_size = data[cursor] as usize;
        cursor += 1;
        
        // Read palette
        let mut palette = Vec::with_capacity(palette_size);
        for _ in 0..palette_size {
            if cursor + 2 > data.len() {
                return Err(PersistenceError::CorruptedData("Incomplete palette".to_string()));
            }
            
            let block_id = u16::from_le_bytes([
                data[cursor], data[cursor + 1]
            ]);
            palette.push(BlockId(block_id));
            cursor += 2;
        }
        
        // Read block indices
        for y in 0..chunk.size() {
            for z in 0..chunk.size() {
                for x in 0..chunk.size() {
                    if cursor >= data.len() {
                        return Err(PersistenceError::CorruptedData("Missing block indices".to_string()));
                    }
                    
                    let index = data[cursor] as usize;
                    if index >= palette.len() {
                        return Err(PersistenceError::CorruptedData("Invalid palette index".to_string()));
                    }
                    
                    chunk.set_block_at(VoxelPos::new(x as i32, y as i32, z as i32), palette[index]);
                    cursor += 1;
                }
            }
        }
        
        Ok(chunk)
    }
    
    /// Read header from data
    fn read_header(&self, data: &[u8]) -> PersistenceResult<ChunkHeader> {
        if data.len() < std::mem::size_of::<ChunkHeader>() {
            return Err(PersistenceError::CorruptedData("Data too small for header".to_string()));
        }
        
        let header_bytes = &data[..std::mem::size_of::<ChunkHeader>()];
        Ok(bincode::deserialize(header_bytes)?)
    }
    
    /// Calculate CRC32 checksum
    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(data);
        hasher.finalize()
    }
    
    /// Analyze chunk to determine best format
    pub fn analyze_chunk(chunk: &Chunk) -> ChunkFormat {
        let mut block_counts = std::collections::HashMap::new();
        let mut last_block = None;
        let mut run_count = 0;
        let total_blocks = chunk.size() * chunk.size() * chunk.size();
        
        for y in 0..chunk.size() {
            for z in 0..chunk.size() {
                for x in 0..chunk.size() {
                    let pos = VoxelPos::new(x as i32, y as i32, z as i32);
                    let block = chunk.get_block_at(pos);
                    
                    *block_counts.entry(block).or_insert(0) += 1;
                    
                    if Some(block) != last_block {
                        run_count += 1;
                        last_block = Some(block);
                    }
                }
            }
        }
        
        let unique_blocks = block_counts.len();
        
        // Use RLE if very few runs (high compression ratio)
        if run_count < total_blocks / 100 {
            ChunkFormat::RLE
        }
        // Use palette if few unique blocks and reasonable number of runs
        else if unique_blocks <= 16 && run_count >= total_blocks / 100 {
            ChunkFormat::Palette
        }
        // Use RLE if decent compression ratio
        else if run_count < total_blocks / 4 {
            ChunkFormat::RLE
        }
        // Otherwise use raw
        else {
            ChunkFormat::Raw
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chunk_serialization_raw() {
        let mut chunk = Chunk::new(ChunkPos { x: 0, y: 0, z: 0 }, 32);
        chunk.set_block_at(VoxelPos::new(10, 20, 30), BlockId(42));
        
        let serializer = ChunkSerializer::new(ChunkFormat::Raw);
        let data = serializer.serialize(&chunk).expect("Chunk serialization should succeed");
        let deserialized = serializer.deserialize(&data).expect("Chunk deserialization should succeed");
        
        assert_eq!(chunk.get_block_at(VoxelPos::new(10, 20, 30)), 
                   deserialized.get_block_at(VoxelPos::new(10, 20, 30)));
    }
    
    #[test]
    fn test_chunk_serialization_rle() {
        let chunk = Chunk::new(ChunkPos { x: 0, y: 0, z: 0 }, 32);
        
        let serializer = ChunkSerializer::new(ChunkFormat::RLE);
        let data = serializer.serialize(&chunk).expect("RLE chunk serialization should succeed");
        let deserialized = serializer.deserialize(&data).expect("RLE chunk deserialization should succeed");
        
        assert_eq!(chunk.position(), deserialized.position());
    }
    
    #[test]
    fn test_chunk_format_analysis() {
        let mut chunk = Chunk::new(ChunkPos { x: 0, y: 0, z: 0 }, 32);
        
        // Empty chunk should use RLE
        assert_eq!(ChunkSerializer::analyze_chunk(&chunk), ChunkFormat::RLE);
        
        // Chunk with few block types should use Palette
        // Fill a substantial portion with a few block types
        for y in 0..16 {
            for x in 0..16 {
                for z in 0..16 {
                    let block_type = ((x + y + z) % 4) as u16;
                    chunk.set_block_at(VoxelPos::new(x, y, z), BlockId(block_type));
                }
            }
        }
        assert_eq!(ChunkSerializer::analyze_chunk(&chunk), ChunkFormat::Palette);
    }

    #[test]
    fn test_corruption_detection_invalid_magic() {
        let mut corrupted_data = vec![0u8; 100];
        // Set invalid magic bytes (should be "ECNK")
        corrupted_data[0..4].copy_from_slice(b"FAKE");
        
        let serializer = ChunkSerializer::new(ChunkFormat::Raw);
        let result = serializer.deserialize(&corrupted_data);
        
        assert!(result.is_err());
        match result.expect_err("Should have failed with corrupted data error") {
            PersistenceError::CorruptedData(msg) => {
                assert!(msg.contains("Invalid chunk magic"));
            }
            _ => panic!("Expected CorruptedData error"),
        }
    }

    #[test]
    fn test_corruption_detection_invalid_size() {
        let too_small_data = vec![0u8; 10]; // Too small for even a header
        
        let serializer = ChunkSerializer::new(ChunkFormat::Raw);
        let result = serializer.deserialize(&too_small_data);
        
        assert!(result.is_err());
        match result.expect_err("Should have failed with corrupted data error") {
            PersistenceError::CorruptedData(msg) => {
                assert!(msg.contains("Data too small"));
            }
            _ => panic!("Expected CorruptedData error"),
        }
    }

    #[test]
    fn test_corruption_detection_invalid_version() {
        let chunk = Chunk::new(ChunkPos { x: 0, y: 0, z: 0 }, 32);
        let serializer = ChunkSerializer::new(ChunkFormat::Raw);
        let mut data = serializer.serialize(&chunk).expect("Chunk serialization should succeed");
        
        // Corrupt the version field (it's after magic bytes at offset 4-7)
        data[4..8].copy_from_slice(&999u32.to_le_bytes());
        
        let result = serializer.deserialize(&data);
        assert!(result.is_err());
        match result.expect_err("Should have failed with version mismatch error") {
            PersistenceError::VersionMismatch { expected, found } => {
                assert_eq!(expected, CHUNK_FORMAT_VERSION);
                assert_eq!(found, 999);
            }
            _ => panic!("Expected VersionMismatch error"),
        }
    }

    #[test]
    fn test_corruption_detection_invalid_block_count() {
        let chunk = Chunk::new(ChunkPos { x: 0, y: 0, z: 0 }, 32);
        let serializer = ChunkSerializer::new(ChunkFormat::Raw);
        let mut data = serializer.serialize(&chunk).expect("Chunk serialization should succeed");
        
        // Find and corrupt the block_count field in the header
        // Create a fake header with invalid block count
        let invalid_header = ChunkHeader {
            magic: [CHUNK_MAGIC[0], CHUNK_MAGIC[1], CHUNK_MAGIC[2], CHUNK_MAGIC[3]],
            version: CHUNK_FORMAT_VERSION,
            format: 0,
            chunk_pos: ChunkPos { x: 0, y: 0, z: 0 },
            block_count: 50_000_000, // Way too many blocks for a 32x32x32 chunk
            timestamp: 0,
            checksum: 0,
        };
        
        let header_bytes = bincode::serialize(&invalid_header).expect("Header serialization should succeed");
        data[0..header_bytes.len()].copy_from_slice(&header_bytes);
        
        let result = serializer.deserialize(&data);
        assert!(result.is_err());
        match result.expect_err("Should have failed with corrupted data error") {
            PersistenceError::CorruptedData(msg) => {
                assert!(msg.contains("Invalid block count"));
            }
            _ => panic!("Expected CorruptedData error with invalid block count"),
        }
    }

    #[test]
    fn test_corruption_detection_checksum_mismatch() {
        let chunk = Chunk::new(ChunkPos { x: 0, y: 0, z: 0 }, 32);
        let serializer = ChunkSerializer::new(ChunkFormat::Raw);
        let mut data = serializer.serialize(&chunk).expect("Chunk serialization should succeed");
        
        // Corrupt some data after the header to cause checksum mismatch
        if data.len() > 50 {
            let idx = data.len() - 1;
            data[idx] = data[idx].wrapping_add(1);
        }
        
        let result = serializer.deserialize(&data);
        assert!(result.is_err());
        match result.expect_err("Should have failed with corrupted data error") {
            PersistenceError::CorruptedData(msg) => {
                assert!(msg.contains("Checksum mismatch"));
            }
            _ => panic!("Expected CorruptedData error with checksum mismatch"),
        }
    }
}