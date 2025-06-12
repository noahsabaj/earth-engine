# Queue Processing Fixes for ParallelChunkManager

## Summary of Changes

Following Data-Oriented Programming (DOP) principles, I've implemented the following fixes to improve queue processing in `parallel_chunk_manager.rs`:

### 1. Increased Consumption Rate
- Changed `max_completions` from `batch_size` to an adaptive value based on queue depth
- Minimum consumption rate is now `(batch_size * 2).max(8)` to ensure it's always higher than generation rate
- Can process up to 64 chunks per frame when the completed queue is severely backed up

### 2. Dynamic Batch Size
- Added `calculate_dynamic_batch_size()` method that adjusts batch size based on queue depths
- Increases batch size when generation queue is filling up (2x when > 4x base batch)
- Decreases batch size when completed queue is backing up to prevent overwhelming the system

### 3. Queue Health Monitoring
- Added `QueueMetrics` struct to track:
  - Queue lengths and usage percentages
  - Current vs dynamic batch sizes
  - Max queue capacity
- Added `check_queue_health()` method that logs warnings when queues exceed 75% capacity
- Added periodic metrics logging in `update_loaded_chunks()`

### 4. Adaptive Processing
- `process_generation_queue()` now uses dynamic batch sizing
- Emergency mode processes all pending chunks when generation queue > 80% full
- Reduces generation when completed queue > 60% full

### 5. New Public Methods
- `get_queue_metrics()` - Returns current queue health metrics
- `get_completed_queue_length()` - Get number of chunks waiting to be consumed
- `log_queue_stats()` - Logs comprehensive queue statistics
- `get_queue_diagnostics()` - Returns formatted diagnostics string
- `are_queues_healthy()` - Check if queues are below warning thresholds

## Key Improvements

1. **Consumption Rate**: Always processes at least 2x the generation rate, scaling up to 4x when needed
2. **Adaptive Behavior**: System automatically adjusts to queue pressure
3. **Early Warning**: Logs warnings at 75% capacity before hitting limits
4. **Emergency Handling**: Special behavior when queues are critically full
5. **Better Observability**: Comprehensive metrics for monitoring queue health

## Testing

Created test file `tests/test_parallel_chunk_manager.rs` with tests for:
- Queue consumption rate verification
- Adaptive batch sizing behavior
- Queue health warning system

The system now ensures that completed chunks are consumed fast enough to prevent queue backup while maintaining smooth frame rates through adaptive processing limits.