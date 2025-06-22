use super::{
    collision_data::CollisionStats, physics_tables, CollisionData, ContactPair, ContactPoint,
    EntityId, PhysicsData, SpatialHash,
};
use crate::physics::error::PhysicsDataResult;
use crate::thread_pool::{PoolCategory, ThreadPoolManager};
use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Physics update to be applied after parallel computation
#[derive(Debug, Clone)]
enum PhysicsUpdate {
    Velocity(usize, [f32; 3]),
    Position(usize, [f32; 3]),
}

/// Integration update containing all changes for an entity
#[derive(Debug, Clone)]
struct IntegrationUpdate {
    index: usize,
    velocity: [f32; 3],
    position: [f32; 3],
    aabb: physics_tables::AABB,
    flags: physics_tables::PhysicsFlags,
}

/// Configuration for the parallel physics solver
#[derive(Debug, Clone)]
pub struct SolverConfig {
    pub worker_threads: usize,
    pub iterations: u32,
    pub position_correction_rate: f32,
    pub velocity_threshold: f32,
    pub sleep_threshold: f32,
    pub batch_size: usize,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get(),
            iterations: 4,
            position_correction_rate: 0.2,
            velocity_threshold: 0.01,
            sleep_threshold: 0.1,
            batch_size: 64,
        }
    }
}

/// Parallel physics solver for data-oriented physics
pub struct ParallelPhysicsSolver {
    config: SolverConfig,
    thread_pool_manager: Arc<ThreadPoolManager>,
    stats: CollisionStats,
}

impl ParallelPhysicsSolver {
    pub fn new(config: SolverConfig) -> PhysicsDataResult<Self> {
        let thread_pool_manager = ThreadPoolManager::global();

        Ok(Self {
            config,
            thread_pool_manager,
            stats: CollisionStats::default(),
        })
    }

    /// Main physics step
    pub fn step(
        &mut self,
        physics_data: &mut PhysicsData,
        collision_data: &mut CollisionData,
        spatial_hash: &SpatialHash,
        dt: f32,
    ) {
        self.stats.reset();

        // Update spatial hash
        self.update_spatial_hash(physics_data, spatial_hash);

        // Broad phase collision detection
        let broad_phase_start = Instant::now();
        self.broad_phase(physics_data, collision_data, spatial_hash);
        self.stats.broad_phase_time_us = broad_phase_start.elapsed().as_micros() as u64;

        // Narrow phase collision detection
        let narrow_phase_start = Instant::now();
        self.narrow_phase(physics_data, collision_data);
        self.stats.narrow_phase_time_us = narrow_phase_start.elapsed().as_micros() as u64;

        // Solve constraints
        let solver_start = Instant::now();
        self.solve_constraints(physics_data, collision_data, dt);
        self.stats.solver_time_us = solver_start.elapsed().as_micros() as u64;

        // Integrate positions
        self.integrate(physics_data, dt);

        // Update GPU buffers if enabled
        if physics_data.gpu_buffers.is_some() {
            // GPU update would happen here
        }
    }

    /// Update spatial hash with current positions
    fn update_spatial_hash(&self, physics_data: &PhysicsData, spatial_hash: &SpatialHash) {
        let count = physics_data.entity_count();

        // Batch updates for better cache efficiency
        let updates: Vec<_> = (0..count)
            .into_par_iter()
            .filter_map(|i| {
                if physics_data.flags[i].is_active() {
                    Some((EntityId(i as u32), physics_data.bounding_boxes[i]))
                } else {
                    None
                }
            })
            .collect();

        spatial_hash.clear();
        spatial_hash.batch_update(&updates);
    }

    /// Broad phase collision detection
    fn broad_phase(
        &mut self,
        physics_data: &PhysicsData,
        collision_data: &mut CollisionData,
        spatial_hash: &SpatialHash,
    ) {
        collision_data.clear();

        // Get all potential collision pairs from spatial hash
        let potential_pairs = spatial_hash.get_all_potential_pairs();
        self.stats.broad_phase_pairs = potential_pairs.len();

        // Check AABB intersections in parallel
        let intersecting_pairs: Vec<_> =
            self.thread_pool_manager.execute(PoolCategory::Physics, || {
                potential_pairs
                    .par_iter()
                    .filter_map(|&(entity_a, entity_b)| {
                        let idx_a = entity_a.index();
                        let idx_b = entity_b.index();

                        // Skip if either entity is inactive
                        if !physics_data.flags[idx_a].is_active()
                            || !physics_data.flags[idx_b].is_active()
                        {
                            return None;
                        }

                        // Skip if both are static
                        if physics_data.flags[idx_a].is_static()
                            && physics_data.flags[idx_b].is_static()
                        {
                            return None;
                        }

                        // Check collision masks
                        if (physics_data.collision_groups[idx_a]
                            & physics_data.collision_masks[idx_b])
                            == 0
                            || (physics_data.collision_groups[idx_b]
                                & physics_data.collision_masks[idx_a])
                                == 0
                        {
                            return None;
                        }

                        // Check AABB intersection
                        let aabb_a = &physics_data.bounding_boxes[idx_a];
                        let aabb_b = &physics_data.bounding_boxes[idx_b];

                        if aabb_a.intersects(aabb_b) {
                            Some((entity_a, entity_b))
                        } else {
                            None
                        }
                    })
                    .collect()
            });

        self.stats.narrow_phase_pairs = intersecting_pairs.len();

        // Store pairs for narrow phase
        for (entity_a, entity_b) in intersecting_pairs {
            collision_data
                .contact_pairs
                .push(ContactPair::new(entity_a, entity_b));
            collision_data.contact_counts.push(0);
        }
    }

    /// Narrow phase collision detection
    fn narrow_phase(&mut self, physics_data: &PhysicsData, collision_data: &mut CollisionData) {
        let contact_count = AtomicU64::new(0);

        // Process collision pairs in parallel batches
        let batches = collision_data.prepare_parallel_batches(self.config.batch_size);

        let contacts: Vec<_> = self.thread_pool_manager.execute(PoolCategory::Physics, || {
            batches
                .par_iter()
                .flat_map(|&(start, end)| {
                    let mut batch_contacts = Vec::new();

                    for pair_idx in start..end {
                        let pair = collision_data.contact_pairs[pair_idx];
                        let idx_a = pair.entity_a.index();
                        let idx_b = pair.entity_b.index();

                        // Simple sphere-sphere collision for now
                        let pos_a = physics_data.positions[idx_a];
                        let pos_b = physics_data.positions[idx_b];

                        let diff = [
                            pos_b[0] - pos_a[0],
                            pos_b[1] - pos_a[1],
                            pos_b[2] - pos_a[2],
                        ];

                        let dist_sq = diff[0] * diff[0] + diff[1] * diff[1] + diff[2] * diff[2];

                        // Assume radius of 0.5 for all entities (simplified)
                        let radius_sum = 1.0;
                        let radius_sum_sq = radius_sum * radius_sum;

                        if dist_sq < radius_sum_sq && dist_sq > 0.0001 {
                            let dist = dist_sq.sqrt();
                            let normal = [diff[0] / dist, diff[1] / dist, diff[2] / dist];

                            let penetration = radius_sum - dist;
                            let contact_pos = [
                                pos_a[0] + normal[0] * 0.5,
                                pos_a[1] + normal[1] * 0.5,
                                pos_a[2] + normal[2] * 0.5,
                            ];

                            let contact = ContactPoint::new(contact_pos, normal, penetration);

                            // Calculate combined material properties
                            let restitution = (physics_data.restitutions[idx_a]
                                + physics_data.restitutions[idx_b])
                                * 0.5;
                            let friction = (physics_data.frictions[idx_a]
                                + physics_data.frictions[idx_b])
                                * 0.5;

                            batch_contacts.push((pair_idx, contact, restitution, friction));
                            contact_count.fetch_add(1, Ordering::Relaxed);
                        }
                    }

                    batch_contacts
                })
                .collect()
        });

        // Add contacts to collision data
        for (pair_idx, contact, restitution, friction) in contacts {
            let pair = collision_data.contact_pairs[pair_idx];
            collision_data.add_collision(
                pair.entity_a,
                pair.entity_b,
                contact,
                restitution,
                friction,
            );
        }

        self.stats.contact_points = contact_count.load(Ordering::Relaxed) as usize;
    }

    /// Solve collision constraints
    fn solve_constraints(
        &mut self,
        physics_data: &mut PhysicsData,
        collision_data: &mut CollisionData,
        dt: f32,
    ) {
        // Calculate relative velocities
        for i in 0..collision_data.pair_count() {
            let pair = collision_data.contact_pairs[i];
            let idx_a = pair.entity_a.index();
            let idx_b = pair.entity_b.index();

            let vel_a = physics_data.velocities[idx_a];
            let vel_b = physics_data.velocities[idx_b];

            collision_data.relative_velocities[i] = [
                vel_b[0] - vel_a[0],
                vel_b[1] - vel_a[1],
                vel_b[2] - vel_a[2],
            ];
        }

        // Iterative constraint solver
        for _ in 0..self.config.iterations {
            self.solve_iteration(physics_data, collision_data, dt);
        }
    }

    /// Single iteration of constraint solving
    fn solve_iteration(
        &self,
        physics_data: &mut PhysicsData,
        collision_data: &CollisionData,
        _dt: f32,
    ) {
        // Process collision pairs in parallel batches
        let batches = collision_data.prepare_parallel_batches(self.config.batch_size);

        // Collect all updates in parallel, then apply them sequentially
        let updates: Vec<_> = self.thread_pool_manager.execute(PoolCategory::Physics, || {
            batches
                .par_iter()
                .flat_map(|&(start, end)| {
                    let mut batch_updates = Vec::new();

                    for pair_idx in start..end {
                        let pair = collision_data.contact_pairs[pair_idx];
                        let idx_a = pair.entity_a.index();
                        let idx_b = pair.entity_b.index();

                        // Skip if both are static
                        if physics_data.flags[idx_a].is_static()
                            && physics_data.flags[idx_b].is_static()
                        {
                            continue;
                        }

                        let contacts = collision_data.get_contacts_for_pair(pair_idx);
                        let inv_mass_a = physics_data.inverse_masses[idx_a];
                        let inv_mass_b = physics_data.inverse_masses[idx_b];
                        let restitution = collision_data.combined_restitutions[pair_idx];

                        for contact in contacts {
                            let relative_vel = collision_data.relative_velocities[pair_idx];

                            // Calculate relative velocity along normal
                            let vel_along_normal = relative_vel[0] * contact.normal[0]
                                + relative_vel[1] * contact.normal[1]
                                + relative_vel[2] * contact.normal[2];

                            // Don't resolve if velocities are separating
                            if vel_along_normal > 0.0 {
                                continue;
                            }

                            // Calculate impulse scalar
                            let impulse_scalar =
                                -(1.0 + restitution) * vel_along_normal / (inv_mass_a + inv_mass_b);

                            // Apply impulse
                            let impulse = [
                                impulse_scalar * contact.normal[0],
                                impulse_scalar * contact.normal[1],
                                impulse_scalar * contact.normal[2],
                            ];

                            // Calculate velocity updates
                            if !physics_data.flags[idx_a].is_static() {
                                let vel_delta = [
                                    -impulse[0] * inv_mass_a,
                                    -impulse[1] * inv_mass_a,
                                    -impulse[2] * inv_mass_a,
                                ];
                                batch_updates.push(PhysicsUpdate::Velocity(idx_a, vel_delta));
                            }

                            if !physics_data.flags[idx_b].is_static() {
                                let vel_delta = [
                                    impulse[0] * inv_mass_b,
                                    impulse[1] * inv_mass_b,
                                    impulse[2] * inv_mass_b,
                                ];
                                batch_updates.push(PhysicsUpdate::Velocity(idx_b, vel_delta));
                            }

                            // Position correction
                            let correction =
                                contact.penetration_depth * self.config.position_correction_rate;
                            let pos_impulse = [
                                correction * contact.normal[0],
                                correction * contact.normal[1],
                                correction * contact.normal[2],
                            ];

                            if !physics_data.flags[idx_a].is_static() {
                                let pos_delta = [
                                    -pos_impulse[0] * inv_mass_a / (inv_mass_a + inv_mass_b),
                                    -pos_impulse[1] * inv_mass_a / (inv_mass_a + inv_mass_b),
                                    -pos_impulse[2] * inv_mass_a / (inv_mass_a + inv_mass_b),
                                ];
                                batch_updates.push(PhysicsUpdate::Position(idx_a, pos_delta));
                            }

                            if !physics_data.flags[idx_b].is_static() {
                                let pos_delta = [
                                    pos_impulse[0] * inv_mass_b / (inv_mass_a + inv_mass_b),
                                    pos_impulse[1] * inv_mass_b / (inv_mass_a + inv_mass_b),
                                    pos_impulse[2] * inv_mass_b / (inv_mass_a + inv_mass_b),
                                ];
                                batch_updates.push(PhysicsUpdate::Position(idx_b, pos_delta));
                            }
                        }
                    }

                    batch_updates
                })
                .collect()
        });

        // Apply all updates sequentially (no data races)
        for update in updates {
            match update {
                PhysicsUpdate::Velocity(idx, delta) => {
                    physics_data.velocities[idx][0] += delta[0];
                    physics_data.velocities[idx][1] += delta[1];
                    physics_data.velocities[idx][2] += delta[2];
                }
                PhysicsUpdate::Position(idx, delta) => {
                    physics_data.positions[idx][0] += delta[0];
                    physics_data.positions[idx][1] += delta[1];
                    physics_data.positions[idx][2] += delta[2];
                }
            }
        }
    }

    /// Integrate velocities and positions
    fn integrate(&self, physics_data: &mut PhysicsData, dt: f32) {
        let count = physics_data.entity_count();

        // Collect integration updates in parallel
        let updates: Vec<_> = self.thread_pool_manager.execute(PoolCategory::Physics, || {
            (0..count)
                .into_par_iter()
                .filter_map(|i| {
                    if physics_data.flags[i].is_active() && physics_data.flags[i].is_dynamic() {
                        let mut vel = physics_data.velocities[i];
                        let mut pos = physics_data.positions[i];
                        let mut flags = physics_data.flags[i];

                        // Apply gravity
                        if flags.has_gravity() {
                            vel[1] += crate::physics::GRAVITY * dt;

                            // Clamp to terminal velocity
                            if vel[1] < crate::physics::TERMINAL_VELOCITY {
                                vel[1] = crate::physics::TERMINAL_VELOCITY;
                            }
                        }

                        // Integrate position
                        pos[0] += vel[0] * dt;
                        pos[1] += vel[1] * dt;
                        pos[2] += vel[2] * dt;

                        // Update bounding box
                        let half_extents = [0.5, 0.5, 0.5]; // Simplified
                        let aabb =
                            physics_tables::AABB::from_center_half_extents(pos, half_extents);

                        // Sleep detection
                        let vel_mag_sq = vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2];

                        if vel_mag_sq < self.config.sleep_threshold * self.config.sleep_threshold {
                            flags.set_flag(physics_tables::PhysicsFlags::SLEEPING, true);
                        } else {
                            flags.set_flag(physics_tables::PhysicsFlags::SLEEPING, false);
                        }

                        Some(IntegrationUpdate {
                            index: i,
                            velocity: vel,
                            position: pos,
                            aabb,
                            flags,
                        })
                    } else {
                        None
                    }
                })
                .collect()
        });

        // Apply all updates sequentially
        for update in updates {
            physics_data.velocities[update.index] = update.velocity;
            physics_data.positions[update.index] = update.position;
            physics_data.bounding_boxes[update.index] = update.aabb;
            physics_data.flags[update.index] = update.flags;
        }
    }

    /// Get collision statistics
    pub fn get_stats(&self) -> &CollisionStats {
        &self.stats
    }
}
