//! Game Gateway System - DOP-style game/engine interface
//!
//! This module provides a type-safe, data-oriented gateway between games and the engine.
//! Games register data buffers and pure functions without using OOP patterns or Any types.
//!
//! Key principles:
//! - No std::any::Any - use concrete data buffers
//! - No methods or self references
//! - Pure functions that transform data
//! - Clear separation between engine and game data

use super::GameContext;
use crate::constants::typed_blocks;
use crate::{BlockId, BlockRegistry, VoxelPos};
use std::sync::Mutex;

// Legacy callback system for compatibility (to be removed)
#[derive(Clone)]
pub struct GameCallbacks {
    pub register_blocks: fn(&mut BlockRegistry),
    pub update_game: fn(&mut dyn std::any::Any, &mut GameContext, f32),
    pub on_block_break: fn(&mut dyn std::any::Any, VoxelPos, BlockId),
    pub on_block_place: fn(&mut dyn std::any::Any, VoxelPos, BlockId),
    pub get_active_block: fn(&dyn std::any::Any) -> BlockId,
}

// ===== Legacy Callback System (for compatibility) =====

impl Default for GameCallbacks {
    fn default() -> Self {
        Self {
            register_blocks: default_register_blocks,
            update_game: default_update_game,
            on_block_break: default_on_block_break,
            on_block_place: default_on_block_place,
            get_active_block: default_get_active_block,
        }
    }
}

fn default_register_blocks(_registry: &mut BlockRegistry) {}
fn default_update_game(_game: &mut dyn std::any::Any, _ctx: &mut GameContext, _delta: f32) {}
fn default_on_block_break(_game: &mut dyn std::any::Any, _pos: VoxelPos, _block: BlockId) {}
fn default_on_block_place(_game: &mut dyn std::any::Any, _pos: VoxelPos, _block: BlockId) {}
fn default_get_active_block(_game: &dyn std::any::Any) -> BlockId {
    BlockId(typed_blocks::GRASS)
}

/// Global callback storage - thread-safe
static GAME_CALLBACKS: Mutex<Option<GameCallbacks>> = Mutex::new(None);

/// Register game callbacks
/// This should be called once during game initialization
pub fn register_game_callbacks(callbacks: GameCallbacks) {
    let mut guard = GAME_CALLBACKS
        .lock()
        .expect("[GameCallbacks] Failed to acquire callback lock");
    *guard = Some(callbacks);
}

/// Get the registered callbacks, or defaults if none registered
pub fn get_game_callbacks() -> GameCallbacks {
    let guard = GAME_CALLBACKS
        .lock()
        .expect("[GameCallbacks] Failed to acquire callback lock");
    guard.clone().unwrap_or_default()
}

/// Execute block registration through callbacks
pub fn execute_register_blocks(registry: &mut BlockRegistry) {
    let callbacks = get_game_callbacks();
    (callbacks.register_blocks)(registry);
}

/// Execute game update through callbacks
pub fn execute_update_game(game: &mut dyn std::any::Any, ctx: &mut GameContext, delta: f32) {
    let callbacks = get_game_callbacks();
    (callbacks.update_game)(game, ctx, delta);
}

/// Execute block break through callbacks
pub fn execute_on_block_break(game: &mut dyn std::any::Any, pos: VoxelPos, block: BlockId) {
    let callbacks = get_game_callbacks();
    (callbacks.on_block_break)(game, pos, block);
}

/// Execute block place through callbacks
pub fn execute_on_block_place(game: &mut dyn std::any::Any, pos: VoxelPos, block: BlockId) {
    let callbacks = get_game_callbacks();
    (callbacks.on_block_place)(game, pos, block);
}

/// Execute get active block through callbacks
pub fn execute_get_active_block(game: &dyn std::any::Any) -> BlockId {
    let callbacks = get_game_callbacks();
    (callbacks.get_active_block)(game)
}
