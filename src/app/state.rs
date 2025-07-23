//! Application state management
//! 
//! Handles screen transitions, navigation logic, and keyboard event processing
//! for the TUI application.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Application screens/states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    /// Main menu screen with Start Test, View Results, Settings, Exit
    Start,
    /// Configuration screen for benchmark parameters
    Config,
    /// Running benchmark with live metrics
    Running,
    /// Results display screen
    Results,
    /// Historical results view
    History,
    /// Settings screen
    Settings,
    /// Exit confirmation or immediate exit
    Exit,
}

impl Default for AppState {
    fn default() -> Self {
        Self::Start
    }
}

/// Navigation actions that can be triggered by keyboard input
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationAction {
    /// Move selection up (arrow up, k)
    Up,
    /// Move selection down (arrow down, j)
    Down,
    /// Move selection left (arrow left, h)
    Left,
    /// Move selection right (arrow right, l)
    Right,
    /// Confirm selection (Enter, Space)
    Select,
    /// Go back/cancel (Esc, Backspace)
    Back,
    /// Next item (Tab)
    Next,
    /// Previous item (Shift+Tab)
    Previous,
    /// Quit application (q, Q, Ctrl+C)
    Quit,
    /// No action
    None,
}

/// Application state manager
#[derive(Debug)]
pub struct StateManager {
    current_state: AppState,
    previous_state: Option<AppState>,
    should_quit: bool,
}

impl StateManager {
    /// Create a new state manager starting at the main menu
    pub fn new() -> Self {
        Self {
            current_state: AppState::Start,
            previous_state: None,
            should_quit: false,
        }
    }

    /// Get the current application state
    pub fn current_state(&self) -> &AppState {
        &self.current_state
    }

    /// Get the previous state if available
    pub fn previous_state(&self) -> Option<&AppState> {
        self.previous_state.as_ref()
    }

    /// Check if the application should quit
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Set the quit flag
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: AppState) {
        if new_state != self.current_state {
            self.previous_state = Some(self.current_state.clone());
            self.current_state = new_state;
        }
    }

    /// Go back to the previous state if available, otherwise go to Start
    pub fn go_back(&mut self) {
        match self.previous_state.take() {
            Some(prev_state) => {
                self.current_state = prev_state;
            }
            None => {
                self.current_state = AppState::Start;
            }
        }
    }

    /// Handle state transitions based on current state and navigation action
    pub fn handle_navigation(&mut self, action: NavigationAction) {
        match action {
            NavigationAction::Quit => {
                self.should_quit = true;
                return;
            }
            NavigationAction::Back => {
                match self.current_state {
                    AppState::Start => {
                        self.should_quit = true;
                    }
                    _ => {
                        self.go_back();
                    }
                }
                return;
            }
            _ => {}
        }

        // Handle state-specific navigation
        match (&self.current_state, action) {
            // From Start screen
            (AppState::Start, NavigationAction::Select) => {
                // This will be handled by the specific screen component
                // based on which menu item is selected
            }
            
            // From Config screen
            (AppState::Config, NavigationAction::Select) => {
                // Start benchmark - transition to Running
                self.transition_to(AppState::Running);
            }
            
            // From Running screen
            (AppState::Running, NavigationAction::Select) => {
                // Benchmark completed - transition to Results
                self.transition_to(AppState::Results);
            }
            
            // From Results screen
            (AppState::Results, NavigationAction::Select) => {
                // Save results and go back to Start
                self.transition_to(AppState::Start);
            }
            
            // From History screen
            (AppState::History, NavigationAction::Select) => {
                // View specific result details
                self.transition_to(AppState::Results);
            }
            
            // From Settings screen - go back to Start
            (AppState::Settings, NavigationAction::Select) => {
                self.transition_to(AppState::Start);
            }
            
            _ => {
                // Other navigation actions (Up, Down, Left, Right, Next, Previous)
                // are handled by individual screen components
            }
        }
    }

    /// Convert keyboard event to navigation action
    pub fn key_to_navigation(key: KeyEvent) -> NavigationAction {
        match key.code {
            // Quit keys
            KeyCode::Char('q') | KeyCode::Char('Q') => NavigationAction::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                NavigationAction::Quit
            }
            
            // Navigation keys
            KeyCode::Up | KeyCode::Char('k') => NavigationAction::Up,
            KeyCode::Down | KeyCode::Char('j') => NavigationAction::Down,
            KeyCode::Left | KeyCode::Char('h') => NavigationAction::Left,
            KeyCode::Right | KeyCode::Char('l') => NavigationAction::Right,
            
            // Selection and confirmation
            KeyCode::Enter | KeyCode::Char(' ') => NavigationAction::Select,
            
            // Back/cancel
            KeyCode::Esc | KeyCode::Backspace => NavigationAction::Back,
            
            // Tab navigation
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    NavigationAction::Previous
                } else {
                    NavigationAction::Next
                }
            }
            
            _ => NavigationAction::None,
        }
    }

    /// Handle a keyboard event and update state accordingly
    pub fn handle_key_event(&mut self, key: KeyEvent) {
        let action = Self::key_to_navigation(key);
        self.handle_navigation(action);
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_state_manager_creation() {
        let state_manager = StateManager::new();
        assert_eq!(*state_manager.current_state(), AppState::Start);
        assert!(!state_manager.should_quit());
        assert!(state_manager.previous_state().is_none());
    }

    #[test]
    fn test_state_transitions() {
        let mut state_manager = StateManager::new();
        
        // Transition to Config
        state_manager.transition_to(AppState::Config);
        assert_eq!(*state_manager.current_state(), AppState::Config);
        assert_eq!(state_manager.previous_state(), Some(&AppState::Start));
        
        // Transition to Running
        state_manager.transition_to(AppState::Running);
        assert_eq!(*state_manager.current_state(), AppState::Running);
        assert_eq!(state_manager.previous_state(), Some(&AppState::Config));
    }

    #[test]
    fn test_go_back() {
        let mut state_manager = StateManager::new();
        
        // Go to Config, then back
        state_manager.transition_to(AppState::Config);
        state_manager.go_back();
        assert_eq!(*state_manager.current_state(), AppState::Start);
        assert!(state_manager.previous_state().is_none());
        
        // Go back from Start (should stay at Start)
        state_manager.go_back();
        assert_eq!(*state_manager.current_state(), AppState::Start);
    }

    #[test]
    fn test_quit_handling() {
        let mut state_manager = StateManager::new();
        
        state_manager.quit();
        assert!(state_manager.should_quit());
        
        // Test quit via navigation
        let mut state_manager2 = StateManager::new();
        state_manager2.handle_navigation(NavigationAction::Quit);
        assert!(state_manager2.should_quit());
    }

    #[test]
    fn test_key_to_navigation() {
        // Test quit keys
        assert_eq!(
            StateManager::key_to_navigation(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            NavigationAction::Quit
        );
        assert_eq!(
            StateManager::key_to_navigation(KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::NONE)),
            NavigationAction::Quit
        );
        assert_eq!(
            StateManager::key_to_navigation(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            NavigationAction::Quit
        );
        
        // Test navigation keys
        assert_eq!(
            StateManager::key_to_navigation(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            NavigationAction::Up
        );
        assert_eq!(
            StateManager::key_to_navigation(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)),
            NavigationAction::Up
        );
        assert_eq!(
            StateManager::key_to_navigation(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            NavigationAction::Down
        );
        assert_eq!(
            StateManager::key_to_navigation(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
            NavigationAction::Down
        );
        
        // Test selection keys
        assert_eq!(
            StateManager::key_to_navigation(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            NavigationAction::Select
        );
        assert_eq!(
            StateManager::key_to_navigation(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
            NavigationAction::Select
        );
        
        // Test back keys
        assert_eq!(
            StateManager::key_to_navigation(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            NavigationAction::Back
        );
        
        // Test tab navigation
        assert_eq!(
            StateManager::key_to_navigation(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
            NavigationAction::Next
        );
        assert_eq!(
            StateManager::key_to_navigation(KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT)),
            NavigationAction::Previous
        );
    }

    #[test]
    fn test_back_navigation_from_start() {
        let mut state_manager = StateManager::new();
        
        // Back from Start should quit
        state_manager.handle_navigation(NavigationAction::Back);
        assert!(state_manager.should_quit());
    }

    #[test]
    fn test_state_specific_navigation() {
        let mut state_manager = StateManager::new();
        
        // From Config to Running
        state_manager.transition_to(AppState::Config);
        state_manager.handle_navigation(NavigationAction::Select);
        assert_eq!(*state_manager.current_state(), AppState::Running);
        
        // From Running to Results
        state_manager.handle_navigation(NavigationAction::Select);
        assert_eq!(*state_manager.current_state(), AppState::Results);
        
        // From Results back to Start
        state_manager.handle_navigation(NavigationAction::Select);
        assert_eq!(*state_manager.current_state(), AppState::Start);
    }

    #[test]
    fn test_handle_key_event() {
        let mut state_manager = StateManager::new();
        
        // Test quit key
        state_manager.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(state_manager.should_quit());
        
        // Test back key from start
        let mut state_manager2 = StateManager::new();
        state_manager2.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(state_manager2.should_quit());
    }
}