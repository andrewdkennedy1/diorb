//! Integration tests for the start screen functionality

use diorb::app::{screens::StartScreen, AppState, StateManager};

#[test]
fn test_start_screen_integration() {
    // Test that we can create a start screen
    let mut start_screen = StartScreen::new();

    // Test initial state
    assert_eq!(start_screen.selected_disk().to_str().unwrap(), "C:\\");

    // Test navigation
    start_screen.select_next();
    assert_eq!(start_screen.selected_disk().to_str().unwrap(), "D:\\");

    // Test wrap around
    start_screen.select_next();
    assert_eq!(start_screen.selected_disk().to_str().unwrap(), "C:\\");

    // Test reverse navigation
    start_screen.select_previous();
    assert_eq!(start_screen.selected_disk().to_str().unwrap(), "D:\\");
}

#[test]
fn test_state_manager_integration() {
    let mut state_manager = StateManager::new();

    // Test initial state
    assert_eq!(*state_manager.current_state(), AppState::Start);

    // Test transitions
    state_manager.transition_to(AppState::Config);
    assert_eq!(*state_manager.current_state(), AppState::Config);

    state_manager.transition_to(AppState::Running);
    assert_eq!(*state_manager.current_state(), AppState::Running);

    // Test going back
    state_manager.go_back();
    assert_eq!(*state_manager.current_state(), AppState::Config);

    state_manager.go_back();
    assert_eq!(*state_manager.current_state(), AppState::Start);
}
