//! TUI application module
//! 
//! Contains the terminal user interface components, screen management,
//! and application state handling.

pub mod app;
pub mod screens;
pub mod state;
pub mod tui;

pub use app::App;
pub use screens::{StartScreen, RunningScreen, ResultsScreen, ResultAction};
pub use state::{AppState, NavigationAction, StateManager};
pub use tui::Tui;