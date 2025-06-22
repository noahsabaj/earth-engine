use crate::instance::InstanceId;
/// Process Control System
///
/// Handles process interruption, cancellation, and control flow.
/// Manages dependencies between processes.
use crate::process::{ProcessData, ProcessId, ProcessStatus};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Reason for process interruption
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterruptReason {
    /// User manually paused
    UserPaused,

    /// Missing required resources
    ResourceUnavailable(Vec<u32>),

    /// Required tool broken
    ToolBroken(u32),

    /// Environmental condition not met
    EnvironmentChanged(String),

    /// Dependency process failed
    DependencyFailed(ProcessId),

    /// Player moved too far
    PlayerOutOfRange,

    /// Server shutdown/maintenance
    ServerShutdown,

    /// Custom reason
    Custom(String),
}

/// Process control system
pub struct ProcessControl {
    /// Active interrupts per process
    interrupts: HashMap<ProcessId, Vec<InterruptReason>>,

    /// Process dependencies
    dependencies: HashMap<ProcessId, HashSet<ProcessId>>,

    /// Reverse dependencies (who depends on this)
    dependents: HashMap<ProcessId, HashSet<ProcessId>>,

    /// Interrupt handlers
    handlers: Vec<Box<dyn InterruptHandler>>,

    /// Control policies
    policies: ControlPolicies,
}

/// Control policies configuration
#[derive(Clone)]
pub struct ControlPolicies {
    /// Auto-resume when conditions met
    pub auto_resume: bool,

    /// Cancel cascades to dependents
    pub cascade_cancel: bool,

    /// Max interruption time before auto-cancel
    pub max_interrupt_time: u64,

    /// Allow multiple simultaneous processes per player
    pub allow_concurrent: bool,

    /// Max concurrent processes per player
    pub max_concurrent: usize,
}

impl Default for ControlPolicies {
    fn default() -> Self {
        Self {
            auto_resume: true,
            cascade_cancel: true,
            max_interrupt_time: 3600 * 20, // 1 hour game time
            allow_concurrent: true,
            max_concurrent: 5,
        }
    }
}

impl ProcessControl {
    pub fn new() -> Self {
        Self {
            interrupts: HashMap::new(),
            dependencies: HashMap::new(),
            dependents: HashMap::new(),
            handlers: Vec::new(),
            policies: ControlPolicies::default(),
        }
    }

    /// Interrupt a process
    pub fn interrupt_process(
        &mut self,
        process_id: ProcessId,
        reason: InterruptReason,
        data: &mut ProcessData,
    ) -> Result<(), String> {
        let index = data
            .find_index(process_id)
            .ok_or_else(|| "Process not found".to_string())?;

        if data.status[index] != ProcessStatus::Active {
            return Err("Process not active".to_string());
        }

        // Pause the process
        data.pause(index);

        // Record interrupt
        self.interrupts
            .entry(process_id)
            .or_insert_with(Vec::new)
            .push(reason.clone());

        // Notify handlers
        for handler in &self.handlers {
            handler.on_interrupt(process_id, &reason);
        }

        // Check cascade
        if matches!(reason, InterruptReason::DependencyFailed(_)) && self.policies.cascade_cancel {
            self.cascade_interrupt(process_id, data);
        }

        Ok(())
    }

    /// Resume an interrupted process
    pub fn resume_process(
        &mut self,
        process_id: ProcessId,
        data: &mut ProcessData,
    ) -> Result<(), String> {
        let index = data
            .find_index(process_id)
            .ok_or_else(|| "Process not found".to_string())?;

        if data.status[index] != ProcessStatus::Paused {
            return Err("Process not paused".to_string());
        }

        // Check if all interrupts cleared
        if let Some(interrupts) = self.interrupts.get(&process_id) {
            if !interrupts.is_empty() {
                return Err(format!("Process still has {} interrupts", interrupts.len()));
            }
        }

        // Resume
        data.resume(index);
        self.interrupts.remove(&process_id);

        Ok(())
    }

    /// Cancel a process
    pub fn cancel_process(
        &mut self,
        process_id: ProcessId,
        data: &mut ProcessData,
    ) -> Result<(), String> {
        let index = data
            .find_index(process_id)
            .ok_or_else(|| "Process not found".to_string())?;

        // Cancel the process
        data.cancel(index);

        // Remove interrupts
        self.interrupts.remove(&process_id);

        // Handle dependents
        if self.policies.cascade_cancel {
            self.cascade_cancel(process_id, data);
        }

        Ok(())
    }

    /// Add process dependency
    pub fn add_dependency(&mut self, process: ProcessId, depends_on: ProcessId) {
        self.dependencies
            .entry(process)
            .or_insert_with(HashSet::new)
            .insert(depends_on);

        self.dependents
            .entry(depends_on)
            .or_insert_with(HashSet::new)
            .insert(process);
    }

    /// Check if process can start
    pub fn can_start(&self, process: ProcessId, data: &ProcessData) -> Result<(), String> {
        // Check dependencies
        if let Some(deps) = self.dependencies.get(&process) {
            for &dep in deps {
                if let Some(dep_index) = data.find_index(dep) {
                    if data.status[dep_index] != ProcessStatus::Completed {
                        return Err(format!("Dependency {:?} not complete", dep));
                    }
                } else {
                    return Err(format!("Dependency {:?} not found", dep));
                }
            }
        }

        Ok(())
    }

    /// Clear specific interrupt
    pub fn clear_interrupt(&mut self, process_id: ProcessId, reason: &InterruptReason) -> bool {
        if let Some(interrupts) = self.interrupts.get_mut(&process_id) {
            let len_before = interrupts.len();
            interrupts.retain(|r| r != reason);
            len_before != interrupts.len()
        } else {
            false
        }
    }

    /// Auto-resume check
    pub fn check_auto_resume(&mut self, data: &mut ProcessData) {
        if !self.policies.auto_resume {
            return;
        }

        let mut to_resume = Vec::new();

        for (&process_id, interrupts) in &self.interrupts {
            if interrupts.is_empty() {
                to_resume.push(process_id);
            }
        }

        for process_id in to_resume {
            let _ = self.resume_process(process_id, data);
        }
    }

    /// Cascade interrupt to dependents
    fn cascade_interrupt(&mut self, failed: ProcessId, data: &mut ProcessData) {
        if let Some(dependents) = self.dependents.get(&failed).cloned() {
            for dependent in dependents {
                let _ = self.interrupt_process(
                    dependent,
                    InterruptReason::DependencyFailed(failed),
                    data,
                );
            }
        }
    }

    /// Cascade cancellation to dependents
    fn cascade_cancel(&mut self, cancelled: ProcessId, data: &mut ProcessData) {
        if let Some(dependents) = self.dependents.get(&cancelled).cloned() {
            for dependent in dependents {
                let _ = self.cancel_process(dependent, data);
            }
        }
    }

    /// Get player's active process count
    pub fn get_player_process_count(&self, player: InstanceId, data: &ProcessData) -> usize {
        data.owners
            .iter()
            .zip(&data.active)
            .filter(|(&owner, &active)| owner == player && active)
            .count()
    }

    /// Check if player can start new process
    pub fn can_player_start_process(&self, player: InstanceId, data: &ProcessData) -> bool {
        if !self.policies.allow_concurrent {
            return self.get_player_process_count(player, data) == 0;
        }

        self.get_player_process_count(player, data) < self.policies.max_concurrent
    }

    /// Register interrupt handler
    pub fn register_handler(&mut self, handler: Box<dyn InterruptHandler>) {
        self.handlers.push(handler);
    }

    /// Set control policies
    pub fn set_policies(&mut self, policies: ControlPolicies) {
        self.policies = policies;
    }
}

/// Interrupt handler trait
pub trait InterruptHandler: Send + Sync {
    /// Called when process is interrupted
    fn on_interrupt(&self, process: ProcessId, reason: &InterruptReason);

    /// Called when interrupt is cleared
    fn on_interrupt_cleared(&self, process: ProcessId, reason: &InterruptReason);
}

/// Process monitor for automatic interruption
pub struct ProcessMonitor {
    /// Conditions to check
    conditions: Vec<Box<dyn MonitorCondition>>,
}

impl ProcessMonitor {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
        }
    }

    /// Add monitoring condition
    pub fn add_condition(&mut self, condition: Box<dyn MonitorCondition>) {
        self.conditions.push(condition);
    }

    /// Check all conditions
    pub fn check_conditions(&self, data: &mut ProcessData, control: &mut ProcessControl) {
        for i in 0..data.len() {
            if !data.active[i] || data.status[i] != ProcessStatus::Active {
                continue;
            }

            let process_id = data.ids[i];

            for condition in &self.conditions {
                if let Some(reason) = condition.check(i, data) {
                    let _ = control.interrupt_process(process_id, reason, data);
                }
            }
        }
    }
}

/// Monitor condition trait
pub trait MonitorCondition: Send + Sync {
    /// Check if condition triggers interrupt
    fn check(&self, index: usize, data: &ProcessData) -> Option<InterruptReason>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ProcessType;

    #[test]
    fn test_process_interruption() {
        let mut control = ProcessControl::new();
        let mut data = ProcessData::new();

        let id = ProcessId::new();
        let owner = InstanceId::new();
        let index = data.add(id, ProcessType::default(), owner, 100);
        data.status[index] = ProcessStatus::Active;

        // Interrupt process
        let result = control.interrupt_process(id, InterruptReason::UserPaused, &mut data);

        assert!(result.is_ok());
        assert_eq!(data.status[index], ProcessStatus::Paused);
        assert_eq!(control.interrupts[&id].len(), 1);
    }

    #[test]
    fn test_dependency_management() {
        let mut control = ProcessControl::new();
        let mut data = ProcessData::new();

        let process1 = ProcessId::new();
        let process2 = ProcessId::new();
        let owner = InstanceId::new();

        data.add(process1, ProcessType::default(), owner, 100);
        let index2 = data.add(process2, ProcessType::default(), owner, 100);

        // Add dependency
        control.add_dependency(process2, process1);

        // Process 2 can't start yet
        assert!(control.can_start(process2, &data).is_err());

        // Complete process 1
        data.status[0] = ProcessStatus::Completed;

        // Now process 2 can start
        assert!(control.can_start(process2, &data).is_ok());
    }

    #[test]
    fn test_concurrent_limits() {
        let control = ProcessControl::new();
        let mut data = ProcessData::new();
        let player = InstanceId::new();

        // Add processes up to limit
        for _ in 0..5 {
            let id = ProcessId::new();
            data.add(id, ProcessType::default(), player, 100);
        }

        assert_eq!(control.get_player_process_count(player, &data), 5);
        assert!(!control.can_player_start_process(player, &data));
    }
}
