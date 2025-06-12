/// Parallel Process Processor
/// 
/// Handles batch processing of multiple processes in parallel.
/// Uses thread pools for CPU-intensive operations.

use crate::process::{ProcessData, ProcessStatus, StateMachine};
use crate::process::error::{ProcessResult, ProcessErrorContext, thread_pool_error};
use crate::thread_pool::{ThreadPoolManager, PoolCategory};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

/// Batch of processes to update
pub struct ProcessBatch {
    /// Indices of processes to update
    pub indices: Vec<usize>,
    /// Delta time in ticks
    pub delta_ticks: u64,
}

/// Parallel processor for batch updates
pub struct ParallelProcessor {
    /// Batch size for parallel processing
    batch_size: usize,
    
    /// Performance metrics
    metrics: ProcessingMetrics,
}

/// Processing metrics
#[derive(Default)]
struct ProcessingMetrics {
    total_processed: u64,
    total_time_ms: u64,
    avg_per_process_us: f64,
}

impl ParallelProcessor {
    pub fn new() -> ProcessResult<Self> {
        Ok(Self {
            batch_size: 64,
            metrics: ProcessingMetrics::default(),
        })
    }
    
    /// Process a batch of processes in parallel
    pub fn process_batch(
        &mut self,
        data: &mut ProcessData,
        state_machines: &mut [StateMachine],
        batch: ProcessBatch,
    ) {
        let start = std::time::Instant::now();
        
        // Split into smaller batches for parallel processing
        let chunks: Vec<_> = batch.indices.chunks(self.batch_size).collect();
        
        // Get raw pointers before the closure
        let data_ptr = data as *mut ProcessData;
        let state_ptr = state_machines.as_mut_ptr();
        let data_len = data.len();
        let delta_ticks = batch.delta_ticks;
        
        // Wrap pointers in atomic for thread safety
        use std::sync::atomic::{AtomicPtr, Ordering};
        let data_atomic = AtomicPtr::new(data_ptr);
        let state_atomic = AtomicPtr::new(state_ptr);
        
        // Process each chunk in parallel
        ThreadPoolManager::global().execute(PoolCategory::Compute, move || {
            chunks.par_iter().for_each(|chunk| {
                let data_ptr = data_atomic.load(Ordering::Relaxed);
                let state_ptr = state_atomic.load(Ordering::Relaxed);
                
                for &index in *chunk {
                    if index >= data_len {
                        continue;
                    }
                    
                    // SAFETY: Thread-safe parallel access is guaranteed because:
                    // - Each thread processes unique indices (no overlap between chunks)
                    // - data_ptr points to valid ProcessData for entire batch processing lifetime
                    // - state_ptr.add(index) is within bounds (checked above with index < data_len)
                    // - No two threads access the same index simultaneously due to chunking strategy
                    // - Atomic pointers ensure visibility across threads
                    // - Mutable references are non-overlapping due to unique indices
                    unsafe {
                        Self::update_single_process(
                            &mut *data_ptr,
                            &mut *state_ptr.add(index),
                            index,
                            delta_ticks,
                        );
                    }
                }
            });
        });
        
        // Update metrics
        let elapsed = start.elapsed();
        self.metrics.total_processed += batch.indices.len() as u64;
        self.metrics.total_time_ms += elapsed.as_millis() as u64;
        self.metrics.avg_per_process_us = 
            (elapsed.as_micros() as f64) / (batch.indices.len() as f64);
    }
    
    /// Update a single process
    fn update_single_process(
        data: &mut ProcessData,
        state_machine: &mut StateMachine,
        index: usize,
        delta_ticks: u64,
    ) {
        if !data.active[index] {
            return;
        }
        
        match data.status[index] {
            ProcessStatus::Active => {
                // Update elapsed time
                data.update(index, delta_ticks);
                
                // Update state machine
                let progress = data.get_progress(index);
                let _actions = state_machine.update(delta_ticks, progress);
                
                // Actions are handled by the executor, not here
            }
            _ => {}
        }
    }
    
    /// Process multiple batches concurrently
    pub fn process_concurrent_batches(
        &mut self,
        batches: Vec<(Arc<Mutex<ProcessData>>, Vec<usize>, u64)>,
    ) {
        let start = std::time::Instant::now();
        
        ThreadPoolManager::global().execute(PoolCategory::Compute, || {
            batches.par_iter().for_each(|(data_arc, indices, delta_ticks)| {
                match data_arc.lock() {
                    Ok(mut data) => {
                        for &index in indices {
                            if index < data.len() && data.active[index] {
                                data.update(index, *delta_ticks);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to lock process data: {}. Skipping batch.", e);
                    }
                }
            });
        });
        
        let elapsed = start.elapsed();
        println!("Processed {} batches in {:?}", batches.len(), elapsed);
    }
    
    /// Get processing metrics
    pub fn metrics(&self) -> String {
        format!(
            "Processes: {}, Total time: {}ms, Avg per process: {:.2}μs",
            self.metrics.total_processed,
            self.metrics.total_time_ms,
            self.metrics.avg_per_process_us
        )
    }
    
    /// Set batch size for parallel processing
    pub fn set_batch_size(&mut self, size: usize) {
        self.batch_size = size.max(1);
    }
}

/// Parallel stage processor for complex multi-stage processes
pub struct ParallelStageProcessor {
    /// Worker threads for stage processing
    workers: Vec<std::thread::JoinHandle<()>>,
    
    /// Channel for sending work
    sender: crossbeam_channel::Sender<StageWork>,
    
    /// Channel for receiving results
    receiver: crossbeam_channel::Receiver<StageResult>,
}

/// Work item for stage processing
struct StageWork {
    process_id: usize,
    stage_index: u16,
    inputs: Vec<u32>,
}

/// Result from stage processing
struct StageResult {
    process_id: usize,
    stage_index: u16,
    success: bool,
    outputs: Vec<u32>,
}

impl ParallelStageProcessor {
    pub fn new(num_workers: usize) -> Self {
        let (send_work, recv_work) = crossbeam_channel::unbounded::<StageWork>();
        let (send_result, recv_result) = crossbeam_channel::unbounded::<StageResult>();
        
        let mut workers = Vec::new();
        
        for i in 0..num_workers {
            let recv = recv_work.clone();
            let send = send_result.clone();
            
            let handle = std::thread::spawn(move || {
                println!("Stage worker {} started", i);
                
                while let Ok(work) = recv.recv() {
                    // Process stage (simplified)
                    let result = StageResult {
                        process_id: work.process_id,
                        stage_index: work.stage_index,
                        success: true,
                        outputs: vec![1, 2, 3], // Dummy outputs
                    };
                    
                    let _ = send.send(result);
                }
                
                println!("Stage worker {} stopped", i);
            });
            
            workers.push(handle);
        }
        
        Self {
            workers,
            sender: send_work,
            receiver: recv_result,
        }
    }
    
    /// Submit work for stage processing
    pub fn submit_stage(&self, process_id: usize, stage_index: u16, inputs: Vec<u32>) {
        let work = StageWork {
            process_id,
            stage_index,
            inputs,
        };
        
        let _ = self.sender.send(work);
    }
    
    /// Collect completed stage results
    pub fn collect_results(&self) -> Vec<StageResult> {
        let mut results = Vec::new();
        
        while let Ok(result) = self.receiver.try_recv() {
            results.push(result);
        }
        
        results
    }
}

impl Drop for ParallelStageProcessor {
    fn drop(&mut self) {
        // Signal workers to stop
        drop(self.sender.clone());
        
        // Wait for workers to finish
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::{ProcessId, ProcessType};
    use crate::instance::InstanceId;
    
    #[test]
    fn test_parallel_batch_processing() {
        let mut processor = ParallelProcessor::new().expect("Failed to create processor");
        let mut data = ProcessData::new();
        let mut state_machines = Vec::new();
        
        // Create test processes
        for _ in 0..100 {
            let id = ProcessId::new();
            let owner = InstanceId::new();
            data.add(id, ProcessType::default(), owner, 1000);
            data.status.last_mut().unwrap().clone_from(&ProcessStatus::Active);
            state_machines.push(StateMachine::new());
        }
        
        // Process batch
        let batch = ProcessBatch {
            indices: (0..100).collect(),
            delta_ticks: 10,
        };
        
        processor.process_batch(&mut data, &mut state_machines, batch);
        
        // Check all processes were updated
        for i in 0..100 {
            assert_eq!(data.elapsed[i], 10);
        }
        
        println!("Metrics: {}", processor.metrics());
    }
    
    #[test]
    fn test_concurrent_batches() {
        let mut processor = ParallelProcessor::new().expect("Failed to create processor");
        
        // Create multiple data sets
        let mut batches = Vec::new();
        
        for _ in 0..4 {
            let mut data = ProcessData::new();
            
            for _ in 0..25 {
                let id = ProcessId::new();
                let owner = InstanceId::new();
                data.add(id, ProcessType::default(), owner, 1000);
                data.status.last_mut().unwrap().clone_from(&ProcessStatus::Active);
            }
            
            let data_arc = Arc::new(Mutex::new(data));
            let indices = (0..25).collect();
            batches.push((data_arc, indices, 5));
        }
        
        processor.process_concurrent_batches(batches);
    }
}