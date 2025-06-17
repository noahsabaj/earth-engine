//! Compile-time and runtime validation for GPU types

use encase::ShaderSize;
use crate::gpu::types::{terrain, GpuData};

/// Validate all GPU types at runtime (debug builds only)
/// 
/// This function logs the actual sizes of all GPU types and verifies
/// they meet alignment requirements.
#[cfg(debug_assertions)]
pub fn validate_all_gpu_types() {
    log::info!("[GPU Validation] Starting GPU type validation...");
    
    // Validate terrain types
    terrain::validate_terrain_sizes();
    
    // Future validations:
    // lighting::validate_lighting_sizes();
    // physics::validate_physics_sizes();
    // particles::validate_particle_sizes();
    
    log::info!("[GPU Validation] All GPU types validated successfully!");
}

/// Compile-time validation macro for GPU types
/// 
/// Usage: `validate_gpu_alignment!(BlockDistribution);`
#[macro_export]
macro_rules! validate_gpu_alignment {
    ($type:ty) => {
        const _: () = {
            // This will fail to compile if the type doesn't implement ShaderType
            let _ = <$type as encase::ShaderType>::METADATA;
            
            // Ensure the type also implements our GpuData trait
            fn _check_gpu_data<T: $crate::gpu::types::GpuData>() {}
            let _ = _check_gpu_data::<$type>;
        };
    };
}

/// Runtime size checker for debugging
pub fn check_buffer_size<T: GpuData>(expected: u64) -> Result<(), String> {
    let actual = T::SHADER_SIZE.get() as u64;
    if actual != expected {
        Err(format!(
            "Buffer size mismatch for {}: expected {} bytes, got {} bytes",
            std::any::type_name::<T>(),
            expected,
            actual
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::types::terrain::{BlockDistribution, TerrainParams};
    
    #[test]
    fn test_terrain_types_implement_gpu_data() {
        // These will fail to compile if the types don't implement GpuData
        fn assert_gpu_data<T: GpuData>() {}
        
        assert_gpu_data::<BlockDistribution>();
        assert_gpu_data::<TerrainParams>();
    }
    
    #[test]
    fn test_terrain_alignment() {
        // Verify that types are properly aligned
        let block_size = BlockDistribution::SHADER_SIZE.get();
        let params_size = TerrainParams::SHADER_SIZE.get();
        
        assert!(block_size % 16 == 0, "BlockDistribution must be 16-byte aligned");
        assert!(params_size % 16 == 0, "TerrainParams must be 16-byte aligned");
    }
}