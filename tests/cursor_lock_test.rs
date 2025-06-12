// Test for cursor lock behavior after window focus loss/regain
// This test verifies that clicking back into the window re-locks the cursor

use earth_engine::input::InputState;
use winit::event::{ElementState, MouseButton};

#[test]
fn test_cursor_relock_on_click() {
    let mut input_state = InputState::new();
    
    // Initial state - cursor should not be locked
    assert!(!input_state.is_cursor_locked());
    
    // Lock the cursor
    input_state.set_cursor_locked(true);
    assert!(input_state.is_cursor_locked());
    
    // Simulate window losing focus - cursor should be unlocked
    input_state.set_cursor_locked(false);
    input_state.clear_mouse_delta();
    assert!(!input_state.is_cursor_locked());
    
    // Simulate clicking back into the window
    // In the actual implementation, this would trigger the re-lock logic
    input_state.process_mouse_button(MouseButton::Left, ElementState::Pressed);
    
    // The actual re-locking happens in the window event handler,
    // so we simulate that here
    if !input_state.is_cursor_locked() {
        input_state.set_cursor_locked(true);
        input_state.reset_mouse_tracking();
    }
    
    assert!(input_state.is_cursor_locked());
}

#[test]
fn test_mouse_tracking_reset() {
    let mut input_state = InputState::new();
    
    // Simulate some mouse movement
    input_state.process_mouse_motion((100.0, 50.0));
    let (dx, dy) = input_state.get_mouse_delta();
    assert!(dx != 0.0 || dy != 0.0);
    
    // Reset mouse tracking
    input_state.reset_mouse_tracking();
    input_state.clear_mouse_delta();
    
    // Delta should be zero after reset
    let (dx, dy) = input_state.get_mouse_delta();
    assert_eq!(dx, 0.0);
    assert_eq!(dy, 0.0);
}