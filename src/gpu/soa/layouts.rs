//! SOA memory layout management
//! 
//! Provides tools for analyzing and optimizing Structure of Arrays memory layouts
//! for efficient GPU access patterns and cache utilization.

use std::collections::HashMap;

/// Describes how data should be accessed for optimal performance
#[derive(Debug, Clone)]
pub struct AccessPattern {
    /// Stride between elements in bytes
    pub stride: usize,
    /// Offset from base address in bytes
    pub offset: usize,
    /// Number of contiguous elements to prefetch
    pub prefetch_count: usize,
    /// Whether access is sequential or strided
    pub is_sequential: bool,
}

impl AccessPattern {
    /// Create a sequential access pattern
    pub fn sequential(element_size: usize) -> Self {
        Self {
            stride: element_size,
            offset: 0,
            prefetch_count: 16, // Typical cache line size
            is_sequential: true,
        }
    }
    
    /// Create a strided access pattern
    pub fn strided(stride: usize, offset: usize) -> Self {
        Self {
            stride,
            offset,
            prefetch_count: 1,
            is_sequential: false,
        }
    }
}

/// Field information for SOA layout analysis
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// Field name
    pub name: String,
    /// Size of the field in bytes
    pub size: usize,
    /// Required alignment in bytes
    pub alignment: usize,
    /// Offset within the SOA structure
    pub offset: usize,
    /// Access frequency (0.0 = cold, 1.0 = hot)
    pub access_frequency: f32,
}

/// Manages SOA memory layouts and access patterns
pub struct SoaLayoutManager {
    /// Field information by name
    fields: HashMap<String, FieldInfo>,
    /// Total structure size
    total_size: usize,
    /// Optimal field ordering for cache efficiency
    optimal_order: Vec<String>,
}

impl SoaLayoutManager {
    /// Create a new layout manager
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            total_size: 0,
            optimal_order: Vec::new(),
        }
    }
    
    /// Add a field to the layout
    pub fn add_field(
        &mut self,
        name: impl Into<String>,
        size: usize,
        alignment: usize,
        access_frequency: f32,
    ) {
        let name = name.into();
        let field = FieldInfo {
            name: name.clone(),
            size,
            alignment,
            offset: 0, // Will be calculated in optimize_layout
            access_frequency,
        };
        self.fields.insert(name, field);
    }
    
    /// Calculate optimal layout for BlockDistributionSOA
    pub fn calculate_block_distribution_layout() -> Self {
        let mut manager = Self::new();
        
        // Add fields with their characteristics
        // Hot fields (accessed frequently)
        manager.add_field("count", 4, 4, 1.0);
        manager.add_field("block_ids", 4 * MAX_BLOCK_DISTRIBUTIONS, 4, 0.9);
        manager.add_field("min_heights", 4 * MAX_BLOCK_DISTRIBUTIONS, 4, 1.0);
        manager.add_field("max_heights", 4 * MAX_BLOCK_DISTRIBUTIONS, 4, 1.0);
        
        // Warm fields (accessed sometimes)
        manager.add_field("probabilities", 4 * MAX_BLOCK_DISTRIBUTIONS, 4, 0.6);
        manager.add_field("noise_thresholds", 4 * MAX_BLOCK_DISTRIBUTIONS, 4, 0.5);
        
        manager.optimize_layout();
        manager
    }
    
    /// Optimize field ordering for cache efficiency
    pub fn optimize_layout(&mut self) {
        // Sort fields by access frequency (hot to cold)
        let mut fields: Vec<_> = self.fields.values().cloned().collect();
        fields.sort_by(|a, b| {
            b.access_frequency
                .partial_cmp(&a.access_frequency)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Calculate offsets with proper alignment
        let mut current_offset = 0;
        self.optimal_order.clear();
        
        for field in &mut fields {
            // Align offset
            current_offset = align_up(current_offset, field.alignment);
            field.offset = current_offset;
            current_offset += field.size;
            
            self.optimal_order.push(field.name.clone());
            self.fields.insert(field.name.clone(), field.clone());
        }
        
        // Final alignment to 16 bytes (GPU requirement)
        self.total_size = align_up(current_offset, 16);
    }
    
    /// Get optimized access pattern for specific fields
    pub fn get_access_pattern(&self, field_names: &[&str]) -> Vec<AccessPattern> {
        field_names
            .iter()
            .filter_map(|name| {
                self.fields.get(*name).map(|field| {
                    if field.size <= 64 {
                        // Small fields - prefetch entire array
                        AccessPattern::sequential(field.size / MAX_BLOCK_DISTRIBUTIONS)
                    } else {
                        // Large fields - stream access
                        AccessPattern::strided(
                            field.size / MAX_BLOCK_DISTRIBUTIONS,
                            field.offset,
                        )
                    }
                })
            })
            .collect()
    }
    
    /// Get field information
    pub fn get_field(&self, name: &str) -> Option<&FieldInfo> {
        self.fields.get(name)
    }
    
    /// Get total structure size
    pub fn total_size(&self) -> usize {
        self.total_size
    }
    
    /// Get optimal field ordering
    pub fn optimal_order(&self) -> &[String] {
        &self.optimal_order
    }
    
    /// Calculate cache efficiency score (0.0 - 1.0)
    pub fn cache_efficiency_score(&self) -> f32 {
        // Simple heuristic: hot fields should be close together
        let mut score = 0.0;
        let mut total_weight = 0.0;
        
        for (i, field_name) in self.optimal_order.iter().enumerate() {
            if let Some(field) = self.fields.get(field_name) {
                // Fields accessed frequently should be early in the layout
                let position_score = 1.0 - (i as f32 / self.optimal_order.len() as f32);
                score += field.access_frequency * position_score;
                total_weight += field.access_frequency;
            }
        }
        
        if total_weight > 0.0 {
            score / total_weight
        } else {
            0.0
        }
    }
}

impl Default for SoaLayoutManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Align a value up to the nearest multiple of alignment
fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

use crate::gpu::soa::MAX_BLOCK_DISTRIBUTIONS;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_layout_optimization() {
        let manager = SoaLayoutManager::calculate_block_distribution_layout();
        
        // Verify hot fields come first
        assert!(manager.optimal_order[0] == "count" || manager.optimal_order[0] == "min_heights");
        
        // Verify total size is 16-byte aligned
        assert_eq!(manager.total_size() % 16, 0);
        
        // Verify cache efficiency is good
        assert!(manager.cache_efficiency_score() > 0.7);
    }
    
    #[test]
    fn test_access_patterns() {
        let manager = SoaLayoutManager::calculate_block_distribution_layout();
        let patterns = manager.get_access_pattern(&["min_heights", "max_heights"]);
        
        assert_eq!(patterns.len(), 2);
        for pattern in patterns {
            assert!(pattern.stride > 0);
        }
    }
}