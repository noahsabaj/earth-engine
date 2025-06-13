use crate::streaming::PageTable;
use crate::streaming::error::{StreamingResult, StreamingErrorContext};
use std::collections::VecDeque;

/// Access pattern tracking for predictive loading
#[derive(Debug, Clone)]
pub struct AccessPattern {
    /// Recent positions (circular buffer)
    pub positions: VecDeque<(f32, f32, f32)>,
    
    /// Recent timestamps
    pub timestamps: VecDeque<f64>,
    
    /// Calculated velocity
    pub velocity: (f32, f32, f32),
    
    /// Calculated acceleration
    pub acceleration: (f32, f32, f32),
    
    /// Movement speed
    pub speed: f32,
}

impl AccessPattern {
    pub fn new(capacity: usize) -> Self {
        Self {
            positions: VecDeque::with_capacity(capacity),
            timestamps: VecDeque::with_capacity(capacity),
            velocity: (0.0, 0.0, 0.0),
            acceleration: (0.0, 0.0, 0.0),
            speed: 0.0,
        }
    }
    
    /// Update pattern with new position
    pub fn update(&mut self, position: (f32, f32, f32), timestamp: f64) -> StreamingResult<()> {
        // Add new position
        self.positions.push_back(position);
        self.timestamps.push_back(timestamp);
        
        // Remove old positions
        while self.positions.len() > self.positions.capacity() {
            self.positions.pop_front();
            self.timestamps.pop_front();
        }
        
        // Calculate velocity and acceleration
        if self.positions.len() >= 2 {
            let curr_time = self.timestamps.back().streaming_context("current_timestamp")?;
            let dt = curr_time - self.timestamps[self.timestamps.len() - 2];
            if dt > 0.0 {
                let prev_pos = self.positions[self.positions.len() - 2];
                let curr_pos = self.positions.back().streaming_context("current_position")?;
                
                let new_velocity = (
                    (curr_pos.0 - prev_pos.0) / dt as f32,
                    (curr_pos.1 - prev_pos.1) / dt as f32,
                    (curr_pos.2 - prev_pos.2) / dt as f32,
                );
                
                // Calculate acceleration
                if self.positions.len() >= 3 {
                    self.acceleration = (
                        (new_velocity.0 - self.velocity.0) / dt as f32,
                        (new_velocity.1 - self.velocity.1) / dt as f32,
                        (new_velocity.2 - self.velocity.2) / dt as f32,
                    );
                }
                
                self.velocity = new_velocity;
                self.speed = (new_velocity.0 * new_velocity.0 + 
                             new_velocity.1 * new_velocity.1 + 
                             new_velocity.2 * new_velocity.2).sqrt();
            }
        }
        
        Ok(())
    }
}

/// Predictive loader for world streaming
pub struct PredictiveLoader {
    /// Player access patterns
    player_patterns: Vec<AccessPattern>,
    
    /// Loading radius based on speed
    base_load_radius: f32,
    max_load_radius: f32,
    
    /// Prediction time horizon
    prediction_time: f32,
    
    /// Priority queue for page loads
    load_queue: Vec<LoadRequest>,
}

/// Page load request with priority
#[derive(Debug, Clone)]
pub struct LoadRequest {
    pub page_x: u32,
    pub page_y: u32,
    pub page_z: u32,
    pub priority: f32,
    pub predicted_time: f32,
}

impl PredictiveLoader {
    pub fn new(
        num_players: usize,
        base_load_radius: f32,
        max_load_radius: f32,
    ) -> Self {
        Self {
            player_patterns: vec![AccessPattern::new(20); num_players],
            base_load_radius,
            max_load_radius,
            prediction_time: 2.0, // Predict 2 seconds ahead
            load_queue: Vec::new(),
        }
    }
    
    /// Update player position and calculate predictions
    pub fn update_player(
        &mut self,
        player_id: usize,
        position: (f32, f32, f32),
        timestamp: f64,
        page_table: &PageTable,
    ) {
        if player_id >= self.player_patterns.len() {
            return;
        }
        
        // Update access pattern
        if let Some(pattern) = self.player_patterns.get_mut(player_id) {
            let _ = pattern.update(position, timestamp);
        }
        
        // Clear old load requests
        self.load_queue.clear();
        
        // Calculate dynamic load radius based on speed
        let pattern = match self.player_patterns.get(player_id) {
            Some(p) => p,
            None => return,
        };
        let dynamic_radius = (self.base_load_radius + pattern.speed * 0.5)
            .min(self.max_load_radius);
        
        // Predict future positions
        let predictions = self.predict_positions(player_id, 10);
        
        // Generate load requests for predicted positions
        for (pred_pos, pred_time) in predictions {
            self.generate_load_requests(
                pred_pos,
                dynamic_radius,
                pred_time,
                page_table,
            );
        }
        
        // Sort by priority (highest first)
        self.load_queue.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
    }
    
    /// Predict future positions based on velocity and acceleration
    fn predict_positions(
        &self,
        player_id: usize,
        num_predictions: usize,
    ) -> Vec<((f32, f32, f32), f32)> {
        let pattern = match self.player_patterns.get(player_id) {
            Some(p) => p,
            None => return Vec::new(),
        };
        let mut predictions = Vec::with_capacity(num_predictions);
        
        if let Some(&current_pos) = pattern.positions.back() {
            let dt = self.prediction_time / num_predictions as f32;
            
            for i in 1..=num_predictions {
                let t = i as f32 * dt;
                
                // Position = p0 + v*t + 0.5*a*t^2
                let predicted_pos = (
                    current_pos.0 + pattern.velocity.0 * t + 0.5 * pattern.acceleration.0 * t * t,
                    current_pos.1 + pattern.velocity.1 * t + 0.5 * pattern.acceleration.1 * t * t,
                    current_pos.2 + pattern.velocity.2 * t + 0.5 * pattern.acceleration.2 * t * t,
                );
                
                predictions.push((predicted_pos, t));
            }
        }
        
        predictions
    }
    
    /// Generate load requests for pages around a position
    fn generate_load_requests(
        &mut self,
        position: (f32, f32, f32),
        radius: f32,
        predicted_time: f32,
        page_table: &PageTable,
    ) {
        // Convert position to page coordinates
        let center_page = page_table.voxel_to_page(
            position.0 as u32,
            position.1 as u32,
            position.2 as u32,
        );
        
        // Calculate page radius
        let page_radius = (radius / page_table.page_size as f32).ceil() as i32;
        
        // Check pages in sphere around position
        for dx in -page_radius..=page_radius {
            for dy in -page_radius..=page_radius {
                for dz in -page_radius..=page_radius {
                    let page_x = center_page.0 as i32 + dx;
                    let page_y = center_page.1 as i32 + dy;
                    let page_z = center_page.2 as i32 + dz;
                    
                    // Skip if outside world bounds
                    if page_x < 0 || page_y < 0 || page_z < 0 ||
                       page_x >= page_table.world_size_pages.0 as i32 ||
                       page_y >= page_table.world_size_pages.1 as i32 ||
                       page_z >= page_table.world_size_pages.2 as i32 {
                        continue;
                    }
                    
                    // Calculate distance from center
                    let dist_sq = (dx * dx + dy * dy + dz * dz) as f32;
                    if dist_sq > page_radius as f32 * page_radius as f32 {
                        continue;
                    }
                    
                    // Check if page is already resident
                    if let Some(page_idx) = page_table.page_index(
                        page_x as u32,
                        page_y as u32,
                        page_z as u32,
                    ) {
                        if let Some(entry) = page_table.entries.get(page_idx) {
                            if entry.is_resident() {
                                continue;
                            }
                        }
                    }
                    
                    // Calculate priority based on distance and time
                    let distance = dist_sq.sqrt() * page_table.page_size as f32;
                    let priority = 1000.0 / (distance + 1.0) / (predicted_time + 0.1);
                    
                    self.load_queue.push(LoadRequest {
                        page_x: page_x as u32,
                        page_y: page_y as u32,
                        page_z: page_z as u32,
                        priority,
                        predicted_time,
                    });
                }
            }
        }
    }
    
    /// Get next pages to load
    pub fn get_load_requests(&self, max_requests: usize) -> &[LoadRequest] {
        let end = max_requests.min(self.load_queue.len());
        &self.load_queue[..end]
    }
    
    /// Adaptive loading parameters based on system performance
    pub fn adapt_parameters(&mut self, frame_time_ms: f32, memory_pressure: f32) {
        // Reduce prediction time if frame time is high
        if frame_time_ms > 20.0 {
            self.prediction_time = (self.prediction_time * 0.9).max(0.5);
        } else if frame_time_ms < 10.0 {
            self.prediction_time = (self.prediction_time * 1.1).min(5.0);
        }
        
        // Adjust load radius based on memory pressure
        if memory_pressure > 0.8 {
            self.base_load_radius *= 0.95;
            self.max_load_radius *= 0.95;
        } else if memory_pressure < 0.5 {
            self.base_load_radius *= 1.05;
            self.max_load_radius *= 1.05;
        }
    }
}

/// Movement pattern classifier for advanced prediction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementPattern {
    Stationary,
    Walking,
    Running,
    Flying,
    Teleporting,
}

/// Classify movement pattern from access history
pub fn classify_movement(pattern: &AccessPattern) -> MovementPattern {
    if pattern.speed < 0.1 {
        MovementPattern::Stationary
    } else if pattern.speed < 5.0 {
        MovementPattern::Walking
    } else if pattern.speed < 20.0 {
        MovementPattern::Running
    } else if pattern.speed < 100.0 {
        MovementPattern::Flying
    } else {
        MovementPattern::Teleporting
    }
}

/// Prediction model for different movement patterns
pub struct MovementPredictor {
    /// Neural network weights (future enhancement)
    weights: Vec<f32>,
}

impl MovementPredictor {
    pub fn new() -> Self {
        Self {
            weights: vec![0.0; 100], // Placeholder for ML model
        }
    }
    
    /// Train predictor on access patterns (future enhancement)
    pub fn train(&mut self, _patterns: &[AccessPattern]) {
        // Future: Implement online learning algorithm
    }
    
    /// Predict future trajectory (future enhancement)
    pub fn predict(&self, _pattern: &AccessPattern) -> Vec<(f32, f32, f32)> {
        // Future: Use trained model for prediction
        Vec::new()
    }
}