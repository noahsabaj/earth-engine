use std::collections::HashMap;
use std::time::{Duration, Instant};
use glam::Vec3;
use crate::world::{VoxelPos, BlockId};
use crate::network::packet::MovementState;

/// Anti-cheat detection and validation
pub struct AntiCheat {
    /// Player violation tracking
    violations: HashMap<u32, PlayerViolations>,
    /// Movement validators
    movement_validator: MovementValidator,
    /// Action rate limiter
    rate_limiter: ActionRateLimiter,
    /// Mining validator
    mining_validator: MiningValidator,
    /// Combat validator
    combat_validator: CombatValidator,
}

impl AntiCheat {
    pub fn new() -> Self {
        Self {
            violations: HashMap::new(),
            movement_validator: MovementValidator::new(),
            rate_limiter: ActionRateLimiter::new(),
            mining_validator: MiningValidator::new(),
            combat_validator: CombatValidator::new(),
        }
    }
    
    /// Validate player movement
    pub fn validate_movement(
        &mut self,
        player_id: u32,
        old_pos: Vec3,
        new_pos: Vec3,
        delta_time: f32,
        movement_state: MovementState,
        on_ground: bool,
    ) -> ValidationResult {
        let result = self.movement_validator.validate(
            old_pos, new_pos, delta_time, movement_state, on_ground
        );
        
        if !result.is_valid {
            self.add_violation(player_id, ViolationType::Movement(result.violation_type.clone()));
        }
        
        result
    }
    
    /// Validate block interaction
    pub fn validate_block_interaction(
        &mut self,
        player_id: u32,
        player_pos: Vec3,
        block_pos: VoxelPos,
        interaction_type: InteractionType,
    ) -> bool {
        // Check rate limit
        if !self.rate_limiter.check_action(player_id, ActionType::BlockInteraction) {
            self.add_violation(player_id, ViolationType::RateLimit);
            return false;
        }
        
        // Check reach distance
        let block_center = Vec3::new(
            block_pos.x as f32 + 0.5,
            block_pos.y as f32 + 0.5,
            block_pos.z as f32 + 0.5,
        );
        let distance = (block_center - player_pos).length();
        
        let max_reach = match interaction_type {
            InteractionType::Break => 5.0,
            InteractionType::Place => 5.0,
            InteractionType::Use => 4.5,
        };
        
        if distance > max_reach {
            self.add_violation(player_id, ViolationType::InvalidReach { distance });
            return false;
        }
        
        true
    }
    
    /// Validate mining speed
    pub fn validate_mining(
        &mut self,
        player_id: u32,
        block_id: BlockId,
        tool_effectiveness: f32,
        time_spent: f32,
    ) -> bool {
        let valid = self.mining_validator.validate_mining_time(
            block_id, tool_effectiveness, time_spent
        );
        
        if !valid {
            self.add_violation(player_id, ViolationType::InstantMining);
        }
        
        valid
    }
    
    /// Validate combat action
    pub fn validate_combat(
        &mut self,
        player_id: u32,
        action: CombatAction,
    ) -> bool {
        // Check rate limit
        let action_type = match action {
            CombatAction::Attack { .. } => ActionType::Attack,
            CombatAction::Block => ActionType::Block,
        };
        
        if !self.rate_limiter.check_action(player_id, action_type) {
            self.add_violation(player_id, ViolationType::RateLimit);
            return false;
        }
        
        // Validate specific combat action
        match action {
            CombatAction::Attack { target_pos, player_pos, player_rotation } => {
                self.combat_validator.validate_attack(
                    player_pos, player_rotation, target_pos
                )
            }
            CombatAction::Block => true,
        }
    }
    
    /// Add violation for player
    fn add_violation(&mut self, player_id: u32, violation_type: ViolationType) {
        let violations = self.violations.entry(player_id)
            .or_insert_with(PlayerViolations::new);
        
        violations.add_violation(violation_type);
    }
    
    /// Get player violation score
    pub fn get_violation_score(&self, player_id: u32) -> f32 {
        self.violations.get(&player_id)
            .map(|v| v.get_score())
            .unwrap_or(0.0)
    }
    
    /// Check if player should be kicked
    pub fn should_kick(&self, player_id: u32) -> bool {
        self.get_violation_score(player_id) > 100.0
    }
    
    /// Check if player should be banned
    pub fn should_ban(&self, player_id: u32) -> bool {
        self.get_violation_score(player_id) > 500.0
    }
    
    /// Clear old violations
    pub fn cleanup_old_violations(&mut self) {
        for violations in self.violations.values_mut() {
            violations.cleanup_old();
        }
    }
    
    /// Reset player violations
    pub fn reset_player(&mut self, player_id: u32) {
        self.violations.remove(&player_id);
        self.rate_limiter.reset_player(player_id);
    }
}

/// Player violation tracking
struct PlayerViolations {
    violations: Vec<(Instant, ViolationType)>,
}

impl PlayerViolations {
    fn new() -> Self {
        Self {
            violations: Vec::new(),
        }
    }
    
    fn add_violation(&mut self, violation_type: ViolationType) {
        self.violations.push((Instant::now(), violation_type));
    }
    
    fn get_score(&self) -> f32 {
        let now = Instant::now();
        let mut score = 0.0;
        
        for (time, violation) in &self.violations {
            // Decay older violations
            let age = now.duration_since(*time).as_secs_f32();
            let decay = (-age / 300.0).exp(); // 5 minute half-life
            
            score += violation.severity() * decay;
        }
        
        score
    }
    
    fn cleanup_old(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(1800); // 30 minutes
        self.violations.retain(|(time, _)| *time > cutoff);
    }
}

/// Types of violations
#[derive(Debug, Clone)]
enum ViolationType {
    Movement(MovementViolation),
    InvalidReach { distance: f32 },
    InstantMining,
    RateLimit,
    InvalidPacket,
    NoClip,
}

impl ViolationType {
    fn severity(&self) -> f32 {
        match self {
            ViolationType::Movement(mv) => mv.severity(),
            ViolationType::InvalidReach { distance } => {
                10.0 * (distance - 5.0).max(0.0)
            }
            ViolationType::InstantMining => 50.0,
            ViolationType::RateLimit => 5.0,
            ViolationType::InvalidPacket => 20.0,
            ViolationType::NoClip => 100.0,
        }
    }
}

/// Movement violations
#[derive(Debug, Clone)]
enum MovementViolation {
    TooFast { speed: f32 },
    TooHigh { height: f32 },
    InvalidFlight,
    Teleport { distance: f32 },
}

impl MovementViolation {
    fn severity(&self) -> f32 {
        match self {
            MovementViolation::TooFast { speed } => {
                5.0 * (speed - 15.0).max(0.0)
            }
            MovementViolation::TooHigh { height } => {
                10.0 * (height - 2.0).max(0.0)
            }
            MovementViolation::InvalidFlight => 30.0,
            MovementViolation::Teleport { distance } => {
                20.0 * (distance / 10.0)
            }
        }
    }
}

/// Movement validation result
pub struct ValidationResult {
    pub is_valid: bool,
    pub corrected_position: Option<Vec3>,
    pub violation_type: MovementViolation,
}

/// Movement validator
struct MovementValidator {
    max_walk_speed: f32,
    max_sprint_speed: f32,
    max_fly_speed: f32,
    max_fall_speed: f32,
    max_jump_height: f32,
}

impl MovementValidator {
    fn new() -> Self {
        Self {
            max_walk_speed: 5.0,
            max_sprint_speed: 8.0,
            max_fly_speed: 10.0,
            max_fall_speed: 55.0,
            max_jump_height: 1.5,
        }
    }
    
    fn validate(
        &self,
        old_pos: Vec3,
        new_pos: Vec3,
        delta_time: f32,
        movement_state: MovementState,
        on_ground: bool,
    ) -> ValidationResult {
        let delta = new_pos - old_pos;
        let horizontal_delta = Vec3::new(delta.x, 0.0, delta.z);
        let horizontal_speed = horizontal_delta.length() / delta_time;
        let vertical_speed = (delta.y / delta_time).abs();
        
        // Check teleportation
        if delta.length() > 50.0 {
            return ValidationResult {
                is_valid: false,
                corrected_position: Some(old_pos),
                violation_type: MovementViolation::Teleport { 
                    distance: delta.length() 
                },
            };
        }
        
        // Check horizontal speed
        let max_speed = match movement_state {
            MovementState::Normal | MovementState::Crouching => self.max_walk_speed,
            MovementState::Sprinting => self.max_sprint_speed,
            MovementState::Flying => self.max_fly_speed,
            _ => self.max_walk_speed,
        };
        
        if horizontal_speed > max_speed * 1.1 { // 10% tolerance
            return ValidationResult {
                is_valid: false,
                corrected_position: None,
                violation_type: MovementViolation::TooFast { 
                    speed: horizontal_speed 
                },
            };
        }
        
        // Check vertical movement
        if delta.y > 0.0 && on_ground {
            // Jumping
            if delta.y > self.max_jump_height {
                return ValidationResult {
                    is_valid: false,
                    corrected_position: None,
                    violation_type: MovementViolation::TooHigh { 
                        height: delta.y 
                    },
                };
            }
        } else if vertical_speed > self.max_fall_speed {
            // Falling too fast
            return ValidationResult {
                is_valid: false,
                corrected_position: None,
                violation_type: MovementViolation::TooFast { 
                    speed: vertical_speed 
                },
            };
        }
        
        // Check flying without permission
        if movement_state == MovementState::Flying && !on_ground {
            // TODO: Check if player has flying permission
        }
        
        ValidationResult {
            is_valid: true,
            corrected_position: None,
            violation_type: MovementViolation::TooFast { speed: 0.0 },
        }
    }
}

/// Action rate limiter
struct ActionRateLimiter {
    limits: HashMap<ActionType, RateLimit>,
    player_actions: HashMap<u32, HashMap<ActionType, Vec<Instant>>>,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
enum ActionType {
    BlockInteraction,
    ChatMessage,
    Attack,
    Block,
    ItemUse,
}

struct RateLimit {
    max_per_second: f32,
    burst_size: u32,
}

impl ActionRateLimiter {
    fn new() -> Self {
        let mut limits = HashMap::new();
        
        limits.insert(ActionType::BlockInteraction, RateLimit {
            max_per_second: 10.0,
            burst_size: 5,
        });
        
        limits.insert(ActionType::ChatMessage, RateLimit {
            max_per_second: 2.0,
            burst_size: 3,
        });
        
        limits.insert(ActionType::Attack, RateLimit {
            max_per_second: 4.0,
            burst_size: 2,
        });
        
        limits.insert(ActionType::ItemUse, RateLimit {
            max_per_second: 5.0,
            burst_size: 3,
        });
        
        Self {
            limits,
            player_actions: HashMap::new(),
        }
    }
    
    fn check_action(&mut self, player_id: u32, action_type: ActionType) -> bool {
        let limit = match self.limits.get(&action_type) {
            Some(limit) => limit,
            None => return true,
        };
        
        let player_actions = self.player_actions
            .entry(player_id)
            .or_insert_with(HashMap::new);
        
        let actions = player_actions
            .entry(action_type)
            .or_insert_with(Vec::new);
        
        let now = Instant::now();
        let cutoff = now - Duration::from_secs(1);
        
        // Remove old actions
        actions.retain(|&time| time > cutoff);
        
        // Check rate
        if actions.len() >= limit.burst_size as usize {
            return false;
        }
        
        actions.push(now);
        true
    }
    
    fn reset_player(&mut self, player_id: u32) {
        self.player_actions.remove(&player_id);
    }
}

/// Mining speed validator
struct MiningValidator {
    base_mining_times: HashMap<BlockId, f32>,
}

impl MiningValidator {
    fn new() -> Self {
        let mut times = HashMap::new();
        
        // Example mining times (seconds)
        times.insert(BlockId(1), 0.75);  // Dirt
        times.insert(BlockId(2), 0.6);   // Grass
        times.insert(BlockId(3), 2.25);  // Stone
        times.insert(BlockId(4), 0.3);   // Wood
        times.insert(BlockId(5), 0.6);   // Sand
        
        Self {
            base_mining_times: times,
        }
    }
    
    fn validate_mining_time(
        &self,
        block_id: BlockId,
        tool_effectiveness: f32,
        time_spent: f32,
    ) -> bool {
        let base_time = self.base_mining_times.get(&block_id)
            .copied()
            .unwrap_or(1.0);
        
        let min_time = base_time / tool_effectiveness.max(1.0);
        
        // Allow 10% tolerance for latency
        time_spent >= min_time * 0.9
    }
}

/// Combat validator
struct CombatValidator {
    max_attack_range: f32,
    max_attack_angle: f32,
}

impl CombatValidator {
    fn new() -> Self {
        Self {
            max_attack_range: 4.5,
            max_attack_angle: 60.0, // degrees
        }
    }
    
    fn validate_attack(
        &self,
        player_pos: Vec3,
        player_rotation: glam::Quat,
        target_pos: Vec3,
    ) -> bool {
        // Check range
        let distance = (target_pos - player_pos).length();
        if distance > self.max_attack_range {
            return false;
        }
        
        // Check angle
        let to_target = (target_pos - player_pos).normalize();
        let forward = player_rotation * Vec3::NEG_Z;
        let angle = forward.dot(to_target).acos().to_degrees();
        
        angle <= self.max_attack_angle
    }
}

/// Interaction types
#[derive(Debug, Clone, Copy)]
pub enum InteractionType {
    Break,
    Place,
    Use,
}

/// Combat actions
#[derive(Debug, Clone)]
pub enum CombatAction {
    Attack {
        target_pos: Vec3,
        player_pos: Vec3,
        player_rotation: glam::Quat,
    },
    Block,
}