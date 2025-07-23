//! Terminal management system
//! 
//! Handles crossterm backend initialization, screen management,
//! and keyboard event processing for the TUI application.

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};

/// Terminal wrapper that manages crossterm backend and screen state
pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    should_quit: bool,
    last_tick: Instant,
    tick_rate: Duration,
}

impl Tui {
    /// Create a new TUI instance with crossterm backend
    pub fn new() -> io::Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        
        Ok(Self {
            terminal,
            should_quit: false,
            last_tick: Instant::now(),
            tick_rate: Duration::from_millis(250), // 4 FPS for responsive UI
        })
    }

    /// Initialize terminal with proper setup
    pub fn init(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    /// Restore terminal to original state
    pub fn restore(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    /// Get terminal size for responsive layout handling
    pub fn size(&self) -> io::Result<ratatui::layout::Rect> {
        Ok(self.terminal.size()?)
    }

    /// Check if terminal meets minimum size requirements (80x24)
    pub fn is_size_adequate(&self) -> io::Result<bool> {
        let size = self.size()?;
        Ok(size.width >= 80 && size.height >= 24)
    }

    /// Draw the UI using the provided render function
    pub fn draw<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }

    /// Check if the application should quit
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Set the quit flag
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Handle keyboard events and return the processed event
    pub fn handle_events(&mut self) -> io::Result<Option<KeyEvent>> {
        let timeout = self.tick_rate
            .checked_sub(self.last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                return Ok(Some(key));
            }
        }

        if self.last_tick.elapsed() >= self.tick_rate {
            self.last_tick = Instant::now();
        }

        Ok(None)
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        // Ensure terminal is restored even if restore() wasn't called
        let _ = self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_creation() {
        // Test that TUI can be created without initializing terminal
        let tui = Tui::new();
        assert!(tui.is_ok());
    }

    #[test]
    fn test_quit_flag() {
        let mut tui = Tui::new().unwrap();
        assert!(!tui.should_quit());
        
        tui.quit();
        assert!(tui.should_quit());
    }

    #[test]
    fn test_tick_rate() {
        let tui = Tui::new().unwrap();
        assert_eq!(tui.tick_rate, Duration::from_millis(250));
    }
}