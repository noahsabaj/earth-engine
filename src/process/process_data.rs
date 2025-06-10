/// Process Data Storage
/// 
/// Structure of Arrays storage for all process data.
/// No process objects - just tables of process properties.

use crate::instance::InstanceId;
use crate::process::{ProcessCategory, ProcessPriority, QualityLevel};
use serde::{Serialize, Deserialize};

/// Unique process identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId(pub u64);

impl ProcessId {
    pub fn new() -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

/// Process type definition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessType {
    pub category: ProcessCategory,
    pub sub_type: u16,
}

impl Default for ProcessType {
    fn default() -> Self {
        Self {
            category: ProcessCategory::Crafting,
            sub_type: 0,
        }
    }
}

/// Process status
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    Pending = 0,
    Active = 1,
    Paused = 2,
    Completed = 3,
    Failed = 4,
    Cancelled = 5,
}

/// Core process data (Structure of Arrays)
pub struct ProcessData {
    /// Process IDs (sparse, some may be inactive)
    pub ids: Vec<ProcessId>,
    
    /// Process types
    pub types: Vec<ProcessType>,
    
    /// Process owners (who initiated)
    pub owners: Vec<InstanceId>,
    
    /// Current status
    pub status: Vec<ProcessStatus>,
    
    /// Priority for scheduling
    pub priority: Vec<ProcessPriority>,
    
    /// Start time (game ticks)
    pub start_time: Vec<u64>,
    
    /// Duration (game ticks)
    pub duration: Vec<u64>,
    
    /// Elapsed time (game ticks)
    pub elapsed: Vec<u64>,
    
    /// Pause time accumulated
    pub pause_time: Vec<u64>,
    
    /// Quality modifiers
    pub quality: Vec<QualityLevel>,
    
    /// Input instances (indices into separate storage)
    pub input_start: Vec<u32>,
    pub input_count: Vec<u32>,
    
    /// Output instances (indices into separate storage)
    pub output_start: Vec<u32>,
    pub output_count: Vec<u32>,
    
    /// Active flags
    pub active: Vec<bool>,
}

impl ProcessData {
    pub fn new() -> Self {
        Self {
            ids: Vec::with_capacity(super::MAX_PROCESSES),
            types: Vec::with_capacity(super::MAX_PROCESSES),
            owners: Vec::with_capacity(super::MAX_PROCESSES),
            status: Vec::with_capacity(super::MAX_PROCESSES),
            priority: Vec::with_capacity(super::MAX_PROCESSES),
            start_time: Vec::with_capacity(super::MAX_PROCESSES),
            duration: Vec::with_capacity(super::MAX_PROCESSES),
            elapsed: Vec::with_capacity(super::MAX_PROCESSES),
            pause_time: Vec::with_capacity(super::MAX_PROCESSES),
            quality: Vec::with_capacity(super::MAX_PROCESSES),
            input_start: Vec::with_capacity(super::MAX_PROCESSES),
            input_count: Vec::with_capacity(super::MAX_PROCESSES),
            output_start: Vec::with_capacity(super::MAX_PROCESSES),
            output_count: Vec::with_capacity(super::MAX_PROCESSES),
            active: Vec::with_capacity(super::MAX_PROCESSES),
        }
    }
    
    /// Add a new process
    pub fn add(
        &mut self,
        id: ProcessId,
        process_type: ProcessType,
        owner: InstanceId,
        duration: u64,
    ) -> usize {
        let index = self.ids.len();
        let now = Self::current_tick();
        
        self.ids.push(id);
        self.types.push(process_type);
        self.owners.push(owner);
        self.status.push(ProcessStatus::Pending);
        self.priority.push(ProcessPriority::Normal);
        self.start_time.push(now);
        self.duration.push(duration);
        self.elapsed.push(0);
        self.pause_time.push(0);
        self.quality.push(QualityLevel::Normal);
        self.input_start.push(0);
        self.input_count.push(0);
        self.output_start.push(0);
        self.output_count.push(0);
        self.active.push(true);
        
        index
    }
    
    /// Update process progress
    pub fn update(&mut self, index: usize, delta_ticks: u64) {
        if !self.active[index] || self.status[index] != ProcessStatus::Active {
            return;
        }
        
        self.elapsed[index] += delta_ticks;
        
        // Check completion
        if self.elapsed[index] >= self.duration[index] {
            self.status[index] = ProcessStatus::Completed;
        }
    }
    
    /// Pause a process
    pub fn pause(&mut self, index: usize) {
        if self.status[index] == ProcessStatus::Active {
            self.status[index] = ProcessStatus::Paused;
            self.pause_time[index] = Self::current_tick();
        }
    }
    
    /// Resume a process
    pub fn resume(&mut self, index: usize) {
        if self.status[index] == ProcessStatus::Paused {
            self.status[index] = ProcessStatus::Active;
            let pause_duration = Self::current_tick() - self.pause_time[index];
            self.pause_time[index] = pause_duration;
        }
    }
    
    /// Cancel a process
    pub fn cancel(&mut self, index: usize) {
        self.status[index] = ProcessStatus::Cancelled;
        self.active[index] = false;
    }
    
    /// Get progress as percentage
    pub fn get_progress(&self, index: usize) -> f32 {
        if self.duration[index] == 0 {
            return 1.0;
        }
        
        (self.elapsed[index] as f32 / self.duration[index] as f32).min(1.0)
    }
    
    /// Get remaining time
    pub fn get_time_remaining(&self, index: usize) -> u64 {
        if self.elapsed[index] >= self.duration[index] {
            0
        } else {
            self.duration[index] - self.elapsed[index]
        }
    }
    
    /// Find process index by ID
    pub fn find_index(&self, id: ProcessId) -> Option<usize> {
        self.ids.iter().position(|&pid| pid == id)
    }
    
    /// Get current game tick (placeholder)
    fn current_tick() -> u64 {
        // In real implementation, would get from game time system
        0
    }
    
    /// Number of processes
    pub fn len(&self) -> usize {
        self.ids.len()
    }
}

/// Input/output storage for processes
pub struct ProcessIO {
    /// All input instances
    pub inputs: Vec<InstanceId>,
    
    /// All output instances
    pub outputs: Vec<InstanceId>,
}

impl ProcessIO {
    pub fn new() -> Self {
        Self {
            inputs: Vec::with_capacity(super::MAX_PROCESSES * 4),
            outputs: Vec::with_capacity(super::MAX_PROCESSES * 4),
        }
    }
    
    /// Add inputs for a process
    pub fn add_inputs(&mut self, inputs: Vec<InstanceId>) -> (u32, u32) {
        let start = self.inputs.len() as u32;
        let count = inputs.len() as u32;
        self.inputs.extend(inputs);
        (start, count)
    }
    
    /// Add outputs for a process
    pub fn add_outputs(&mut self, outputs: Vec<InstanceId>) -> (u32, u32) {
        let start = self.outputs.len() as u32;
        let count = outputs.len() as u32;
        self.outputs.extend(outputs);
        (start, count)
    }
    
    /// Get inputs for a process
    pub fn get_inputs(&self, start: u32, count: u32) -> &[InstanceId] {
        let start = start as usize;
        let end = start + count as usize;
        &self.inputs[start..end]
    }
    
    /// Get outputs for a process
    pub fn get_outputs(&self, start: u32, count: u32) -> &[InstanceId] {
        let start = start as usize;
        let end = start + count as usize;
        &self.outputs[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_process_creation() {
        let mut data = ProcessData::new();
        let id = ProcessId::new();
        let owner = InstanceId::new();
        
        let index = data.add(id, ProcessType::default(), owner, 100);
        
        assert_eq!(data.ids[index], id);
        assert_eq!(data.owners[index], owner);
        assert_eq!(data.duration[index], 100);
        assert_eq!(data.status[index], ProcessStatus::Pending);
    }
    
    #[test]
    fn test_process_progress() {
        let mut data = ProcessData::new();
        let id = ProcessId::new();
        let owner = InstanceId::new();
        
        let index = data.add(id, ProcessType::default(), owner, 100);
        data.status[index] = ProcessStatus::Active;
        
        // Update halfway
        data.update(index, 50);
        assert_eq!(data.get_progress(index), 0.5);
        assert_eq!(data.get_time_remaining(index), 50);
        
        // Complete
        data.update(index, 50);
        assert_eq!(data.get_progress(index), 1.0);
        assert_eq!(data.status[index], ProcessStatus::Completed);
    }
}