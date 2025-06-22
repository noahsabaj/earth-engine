use crate::network::packet::MovementState;
use crate::network_constants::MAX_INPUT_BUFFER;
use glam::{Quat, Vec3};
use std::collections::VecDeque;

/// Input state from the player
#[derive(Debug, Clone)]
pub struct PlayerInput {
    pub sequence: u32,
    pub timestamp: f32,
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub crouch: bool,
    pub sprint: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub delta_time: f32,
}

/// Predicted state after applying input
#[derive(Debug, Clone)]
pub struct PredictedState {
    pub sequence: u32,
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Quat,
    pub movement_state: MovementState,
    pub on_ground: bool,
}

/// Client-side prediction system
pub struct ClientPrediction {
    /// Buffer of unacknowledged inputs
    input_buffer: VecDeque<PlayerInput>,
    /// Last acknowledged state from server
    last_server_state: PredictedState,
    /// Current predicted state
    current_state: PredictedState,
    /// Physics simulation for prediction
    physics: PhysicsSimulator,
    /// Smoothing for error correction
    error_smoothing: ErrorSmoothing,
}

impl ClientPrediction {
    pub fn new(initial_position: Vec3) -> Self {
        let initial_state = PredictedState {
            sequence: 0,
            position: initial_position,
            velocity: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            movement_state: MovementState::Normal,
            on_ground: false,
        };

        Self {
            input_buffer: VecDeque::with_capacity(MAX_INPUT_BUFFER),
            last_server_state: initial_state.clone(),
            current_state: initial_state,
            physics: PhysicsSimulator::new(),
            error_smoothing: ErrorSmoothing::new(),
        }
    }

    /// Calculate prediction error
    fn calculate_prediction_error(&self, server_state: &PredictedState) -> f32 {
        (server_state.position - self.current_state.position).length()
    }

    /// Get current predicted state
    pub fn get_state(&self) -> &PredictedState {
        &self.current_state
    }
}

/// Simple physics simulator for prediction
struct PhysicsSimulator {
    gravity: Vec3,
    walk_speed: f32,
    sprint_speed: f32,
    jump_velocity: f32,
    air_control: f32,
    friction: f32,
}

impl PhysicsSimulator {
    fn new() -> Self {
        Self {
            gravity: Vec3::new(0.0, -20.0, 0.0),
            walk_speed: 4.5,
            sprint_speed: 7.0,
            jump_velocity: 8.0,
            air_control: 0.1,
            friction: 10.0,
        }
    }

    fn simulate_input(&self, state: &PredictedState, input: &PlayerInput) -> PredictedState {
        let mut new_state = state.clone();
        new_state.sequence = input.sequence;

        // Update rotation
        new_state.rotation = Quat::from_euler(
            glam::EulerRot::YXZ,
            input.yaw.to_radians(),
            input.pitch.to_radians(),
            0.0,
        );

        // Calculate movement direction
        let mut move_dir = Vec3::ZERO;
        if input.forward {
            move_dir.z -= 1.0;
        }
        if input.backward {
            move_dir.z += 1.0;
        }
        if input.left {
            move_dir.x -= 1.0;
        }
        if input.right {
            move_dir.x += 1.0;
        }

        // Normalize and rotate by yaw
        if move_dir.length_squared() > 0.0 {
            move_dir = move_dir.normalize();
            let yaw_rotation = Quat::from_rotation_y(input.yaw.to_radians());
            move_dir = yaw_rotation * move_dir;
        }

        // Apply movement
        let speed = if input.sprint {
            self.sprint_speed
        } else {
            self.walk_speed
        };
        let control = if state.on_ground {
            1.0
        } else {
            self.air_control
        };

        if state.on_ground {
            // Ground movement
            new_state.velocity.x = move_dir.x * speed;
            new_state.velocity.z = move_dir.z * speed;

            // Jump
            if input.jump {
                new_state.velocity.y = self.jump_velocity;
                new_state.on_ground = false;
            }
        } else {
            // Air movement
            new_state.velocity.x += move_dir.x * speed * control * input.delta_time;
            new_state.velocity.z += move_dir.z * speed * control * input.delta_time;
        }

        // Apply gravity
        new_state.velocity += self.gravity * input.delta_time;

        // Update position
        new_state.position += new_state.velocity * input.delta_time;

        // Simple ground check (would use actual collision in real implementation)
        if new_state.position.y <= 0.0 {
            new_state.position.y = 0.0;
            new_state.velocity.y = 0.0;
            new_state.on_ground = true;
        } else {
            new_state.on_ground = false;
        }

        // Update movement state
        new_state.movement_state = if input.crouch {
            MovementState::Crouching
        } else if input.sprint && move_dir.length_squared() > 0.0 {
            MovementState::Sprinting
        } else {
            MovementState::Normal
        };

        new_state
    }
}

/// Smooths out prediction errors
pub struct ErrorSmoothing {
    error: Vec3,
    smoothing_rate: f32,
}

impl ErrorSmoothing {
    fn new() -> Self {
        Self {
            error: Vec3::ZERO,
            smoothing_rate: 10.0, // Corrections per second
        }
    }
}

/// Server-side move validation
pub struct MoveValidator {
    max_move_delta: f32,
    max_speed: f32,
}

impl MoveValidator {
    pub fn new() -> Self {
        Self {
            max_move_delta: 10.0, // Maximum distance per update
            max_speed: 15.0,      // Maximum possible speed (sprint + some margin)
        }
    }

    /// Validate a player move
    pub fn validate_move(
        &self,
        old_position: Vec3,
        new_position: Vec3,
        delta_time: f32,
        movement_state: MovementState,
    ) -> Result<Vec3, MoveValidationError> {
        let delta = new_position - old_position;
        let distance = delta.length();

        // Check maximum move distance
        if distance > self.max_move_delta {
            return Err(MoveValidationError::TooFarMove { distance });
        }

        // Check speed
        let speed = distance / delta_time;
        if speed > self.max_speed {
            return Err(MoveValidationError::TooFast { speed });
        }

        // Additional checks based on movement state
        match movement_state {
            MovementState::Normal => {
                if speed > 5.0 {
                    return Err(MoveValidationError::InvalidSpeed);
                }
            }
            MovementState::Sprinting => {
                if speed > 8.0 {
                    return Err(MoveValidationError::InvalidSpeed);
                }
            }
            MovementState::Crouching => {
                if speed > 2.0 {
                    return Err(MoveValidationError::InvalidSpeed);
                }
            }
            _ => {}
        }

        Ok(new_position)
    }
}

/// Add a new input and predict the result
pub fn client_prediction_add_input(
    prediction: &mut ClientPrediction,
    input: PlayerInput,
) -> PredictedState {
    // Add to buffer
    prediction.input_buffer.push_back(input.clone());

    // Limit buffer size
    while prediction.input_buffer.len() > MAX_INPUT_BUFFER {
        prediction.input_buffer.pop_front();
    }

    // Apply input to current state
    prediction.current_state = prediction
        .physics
        .simulate_input(&prediction.current_state, &input);

    prediction.current_state.clone()
}

/// Receive server state and reconcile
pub fn client_prediction_receive_server_state(
    prediction: &mut ClientPrediction,
    server_state: PredictedState,
) {
    // Store server state
    prediction.last_server_state = server_state.clone();

    // Remove acknowledged inputs
    while let Some(input) = prediction.input_buffer.front() {
        if input.sequence <= server_state.sequence {
            prediction.input_buffer.pop_front();
        } else {
            break;
        }
    }

    // Calculate prediction error
    let error = prediction.calculate_prediction_error(&server_state);

    // If error is significant, re-simulate from server state
    if error > 0.1 {
        client_prediction_reconcile_from_server(prediction, server_state);
    } else {
        // Small error - use smoothing
        error_smoothing_add_error(
            &mut prediction.error_smoothing,
            server_state.position - prediction.current_state.position,
        );
    }
}

/// Reconcile by re-simulating from server state
pub fn client_prediction_reconcile_from_server(
    prediction: &mut ClientPrediction,
    server_state: PredictedState,
) {
    // Start from server state
    prediction.current_state = server_state;

    // Re-apply all unacknowledged inputs
    let inputs: Vec<PlayerInput> = prediction.input_buffer.iter().cloned().collect();
    for input in inputs {
        prediction.current_state = prediction
            .physics
            .simulate_input(&prediction.current_state, &input);
    }
}

/// Get current predicted position with smoothing applied
pub fn client_prediction_get_position(prediction: &mut ClientPrediction) -> Vec3 {
    prediction.current_state.position
        + error_smoothing_get_correction(&mut prediction.error_smoothing)
}

/// Clear all predictions
pub fn client_prediction_reset(prediction: &mut ClientPrediction, position: Vec3) {
    prediction.input_buffer.clear();
    prediction.current_state.position = position;
    prediction.current_state.velocity = Vec3::ZERO;
    prediction.current_state.sequence = 0;
    prediction.last_server_state = prediction.current_state.clone();
    error_smoothing_reset(&mut prediction.error_smoothing);
}

/// Add error to smoothing system
pub fn error_smoothing_add_error(smoothing: &mut ErrorSmoothing, error: Vec3) {
    smoothing.error = error;
}

/// Get correction from smoothing system
pub fn error_smoothing_get_correction(smoothing: &mut ErrorSmoothing) -> Vec3 {
    // Exponentially decay the error
    let correction = smoothing.error;
    smoothing.error *= 0.9; // Decay factor

    // Return a portion of the error as correction
    correction * 0.1
}

/// Reset smoothing system
pub fn error_smoothing_reset(smoothing: &mut ErrorSmoothing) {
    smoothing.error = Vec3::ZERO;
}

/// Move validation error
#[derive(Debug)]
pub enum MoveValidationError {
    TooFarMove { distance: f32 },
    TooFast { speed: f32 },
    InvalidSpeed,
    InvalidState,
}
