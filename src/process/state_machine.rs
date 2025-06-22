/// Process State Machine
///
/// Time-based state machines for complex processes.
/// States are data, not objects. Transitions are tables.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Process state identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessState(pub u16);

impl ProcessState {
    pub const IDLE: Self = Self(0);
    pub const PREPARING: Self = Self(1);
    pub const PROCESSING: Self = Self(2);
    pub const FINALIZING: Self = Self(3);
    pub const COMPLETE: Self = Self(4);
    pub const ERROR: Self = Self(999);
}

/// State transition condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionCondition {
    /// Time elapsed (ticks)
    TimeElapsed(u64),
    /// Progress reached (0.0-1.0)
    ProgressReached(f32),
    /// External trigger
    Triggered(String),
    /// Resource available
    ResourceAvailable(u32),
    /// Always transition
    Always,
}

/// State transition definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// Source state
    pub from: ProcessState,
    /// Target state
    pub to: ProcessState,
    /// Condition for transition
    pub condition: TransitionCondition,
    /// Priority (higher = checked first)
    pub priority: u8,
    /// Actions to perform on transition
    pub actions: Vec<TransitionAction>,
}

/// Actions performed during transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionAction {
    /// Consume resources
    ConsumeResources(Vec<(u32, u32)>), // (resource_id, amount)
    /// Produce resources
    ProduceResources(Vec<(u32, u32)>),
    /// Apply quality modifier
    ApplyQuality(i8),
    /// Trigger event
    TriggerEvent(String),
    /// Log message
    LogMessage(String),
}

/// State machine for a process
pub struct StateMachine {
    /// Current state
    current: ProcessState,
    /// Time in current state
    state_time: u64,
    /// Transition table
    transitions: Vec<StateTransition>,
    /// State callbacks (as indices)
    state_callbacks: HashMap<ProcessState, Vec<usize>>,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            current: ProcessState::IDLE,
            state_time: 0,
            transitions: Vec::new(),
            state_callbacks: HashMap::new(),
        }
    }

    /// Add a transition
    pub fn add_transition(&mut self, transition: StateTransition) {
        self.transitions.push(transition);
        // Sort by priority
        self.transitions
            .sort_by_key(|t| std::cmp::Reverse(t.priority));
    }

    /// Update state machine
    pub fn update(&mut self, delta_ticks: u64, progress: f32) -> Vec<TransitionAction> {
        self.state_time += delta_ticks;
        let mut actions = Vec::new();

        // Check transitions from current state
        for transition in &self.transitions {
            if transition.from != self.current {
                continue;
            }

            let should_transition = match &transition.condition {
                TransitionCondition::TimeElapsed(time) => self.state_time >= *time,
                TransitionCondition::ProgressReached(target) => progress >= *target,
                TransitionCondition::Always => true,
                _ => false, // External conditions handled elsewhere
            };

            if should_transition {
                // Perform transition
                self.current = transition.to;
                self.state_time = 0;
                actions.extend(transition.actions.clone());
                break; // Only one transition per update
            }
        }

        actions
    }

    /// Force transition to state
    pub fn force_transition(&mut self, state: ProcessState) {
        self.current = state;
        self.state_time = 0;
    }

    /// Get current state
    pub fn current_state(&self) -> ProcessState {
        self.current
    }

    /// Get time in current state
    pub fn state_time(&self) -> u64 {
        self.state_time
    }

    /// Check if in final state
    pub fn is_complete(&self) -> bool {
        self.current == ProcessState::COMPLETE
    }

    /// Check if in error state
    pub fn is_error(&self) -> bool {
        self.current == ProcessState::ERROR
    }
}

/// State machine templates for common processes
pub struct StateMachineTemplates;

impl StateMachineTemplates {
    /// Simple linear process template
    pub fn linear_process(stage_duration: u64) -> StateMachine {
        let mut sm = StateMachine::new();

        // Idle -> Preparing
        sm.add_transition(StateTransition {
            from: ProcessState::IDLE,
            to: ProcessState::PREPARING,
            condition: TransitionCondition::Always,
            priority: 10,
            actions: vec![TransitionAction::LogMessage("Starting process".to_string())],
        });

        // Preparing -> Processing
        sm.add_transition(StateTransition {
            from: ProcessState::PREPARING,
            to: ProcessState::PROCESSING,
            condition: TransitionCondition::TimeElapsed(stage_duration / 4),
            priority: 10,
            actions: vec![TransitionAction::LogMessage("Begin processing".to_string())],
        });

        // Processing -> Finalizing
        sm.add_transition(StateTransition {
            from: ProcessState::PROCESSING,
            to: ProcessState::FINALIZING,
            condition: TransitionCondition::ProgressReached(0.8),
            priority: 10,
            actions: vec![TransitionAction::LogMessage("Finalizing".to_string())],
        });

        // Finalizing -> Complete
        sm.add_transition(StateTransition {
            from: ProcessState::FINALIZING,
            to: ProcessState::COMPLETE,
            condition: TransitionCondition::ProgressReached(1.0),
            priority: 10,
            actions: vec![TransitionAction::LogMessage("Process complete".to_string())],
        });

        sm
    }

    /// Multi-stage crafting template
    pub fn crafting_process() -> StateMachine {
        let mut sm = StateMachine::new();

        // Define crafting-specific states
        const GATHER_MATERIALS: ProcessState = ProcessState(10);
        const HEAT_FORGE: ProcessState = ProcessState(11);
        const SHAPE_ITEM: ProcessState = ProcessState(12);
        const COOL_DOWN: ProcessState = ProcessState(13);
        const POLISH: ProcessState = ProcessState(14);

        // Add transitions
        sm.add_transition(StateTransition {
            from: ProcessState::IDLE,
            to: GATHER_MATERIALS,
            condition: TransitionCondition::Always,
            priority: 10,
            actions: vec![
                TransitionAction::ConsumeResources(vec![(1, 10), (2, 5)]), // Iron, Coal
            ],
        });

        sm.add_transition(StateTransition {
            from: GATHER_MATERIALS,
            to: HEAT_FORGE,
            condition: TransitionCondition::TimeElapsed(20),
            priority: 10,
            actions: vec![],
        });

        sm.add_transition(StateTransition {
            from: HEAT_FORGE,
            to: SHAPE_ITEM,
            condition: TransitionCondition::TimeElapsed(40),
            priority: 10,
            actions: vec![],
        });

        sm.add_transition(StateTransition {
            from: SHAPE_ITEM,
            to: COOL_DOWN,
            condition: TransitionCondition::TimeElapsed(60),
            priority: 10,
            actions: vec![TransitionAction::ApplyQuality(1)],
        });

        sm.add_transition(StateTransition {
            from: COOL_DOWN,
            to: POLISH,
            condition: TransitionCondition::TimeElapsed(30),
            priority: 10,
            actions: vec![],
        });

        sm.add_transition(StateTransition {
            from: POLISH,
            to: ProcessState::COMPLETE,
            condition: TransitionCondition::TimeElapsed(20),
            priority: 10,
            actions: vec![
                TransitionAction::ProduceResources(vec![(100, 1)]), // Sword
                TransitionAction::TriggerEvent("item_crafted".to_string()),
            ],
        });

        sm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_transitions() {
        let mut sm = StateMachine::new();

        sm.add_transition(StateTransition {
            from: ProcessState::IDLE,
            to: ProcessState::PROCESSING,
            condition: TransitionCondition::TimeElapsed(10),
            priority: 10,
            actions: vec![],
        });

        // Should not transition yet
        sm.update(5, 0.0);
        assert_eq!(sm.current_state(), ProcessState::IDLE);

        // Should transition now
        sm.update(5, 0.0);
        assert_eq!(sm.current_state(), ProcessState::PROCESSING);
        assert_eq!(sm.state_time(), 0);
    }

    #[test]
    fn test_progress_transitions() {
        let mut sm = StateMachine::new();

        sm.add_transition(StateTransition {
            from: ProcessState::IDLE,
            to: ProcessState::COMPLETE,
            condition: TransitionCondition::ProgressReached(0.5),
            priority: 10,
            actions: vec![],
        });

        // Should not transition
        sm.update(10, 0.3);
        assert_eq!(sm.current_state(), ProcessState::IDLE);

        // Should transition
        sm.update(10, 0.6);
        assert_eq!(sm.current_state(), ProcessState::COMPLETE);
    }
}
