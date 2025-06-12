# Archived Test Files

This directory contains test files that were created between June 8-11, 2025, during the Earth Engine development process.

## Archive Information

- **Creation Period**: June 8-11, 2025
- **Reason for Archival**: These tests contain compilation errors that prevent the codebase from building cleanly
- **Architecture**: The tests follow Data-Oriented Programming (DOP) patterns as per the project philosophy
- **Status**: Archived until the engine APIs stabilize sufficiently to update the tests

## Contents

The following test files have been archived:
- `chunk_loading_throttle_test.rs` - Tests for chunk loading throttle functionality
- `data_oriented_integration.rs` - Integration tests for data-oriented architecture
- `gpu_driven_integration.rs` - GPU-driven rendering integration tests
- `gpu_memory_cleanup_test.rs` - GPU memory management and cleanup tests
- `mesh_optimization_test.rs` - Mesh optimization pipeline tests
- `optimization_pipeline_integration.rs` - Full optimization pipeline integration tests
- `physics_data_integration.rs` - Physics system data integration tests
- `spatial_index_integration.rs` - Spatial indexing integration tests
- `thread_pool_migration_test.rs` - Thread pool migration and consolidation tests

## Future Use

These tests can be referenced for:
- Understanding test patterns established early in the project
- Extracting useful test cases once APIs stabilize
- Learning from the DOP testing approach used
- Identifying areas that need test coverage in the future

## Note

While these tests don't currently compile, they represent valuable work that demonstrates the testing approach for a data-oriented voxel engine. They should be revisited once the core engine architecture stabilizes and can serve as a foundation for future test development.