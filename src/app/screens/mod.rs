//! TUI screen components
//!
//! Contains individual screen implementations for different application states.

pub mod config;
pub mod history;
pub mod results;
pub mod running;
pub mod start;

pub use config::ConfigScreen;
pub use history::HistoryScreen;
pub use results::{ResultAction, ResultsScreen};
pub use running::RunningScreen;
pub use start::StartScreen;
