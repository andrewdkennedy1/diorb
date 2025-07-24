//! Configuration screen implementation
//!
//! Allows users to select benchmark parameters using drop-down menus
//! and real-time input validation.

use crate::util::units::format_duration;
use crate::{
    app::state::AppState,
    config::{BenchmarkConfig, BenchmarkMode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use std::path::PathBuf;
use std::time::Duration;

/// Represents a single selectable field in the config screen
#[derive(Debug, Clone, PartialEq, Eq)]
enum ConfigField {
    Disk,
    Mode,
    FileSize,
    BlockSize,
    Duration,
    Threads,
}

impl ConfigField {
    fn all() -> Vec<Self> {
        vec![
            Self::Disk,
            Self::Mode,
            Self::FileSize,
            Self::BlockSize,
            Self::Duration,
            Self::Threads,
        ]
    }

    fn title(&self) -> &'static str {
        match self {
            Self::Disk => "Disk",
            Self::Mode => "Mode",
            Self::FileSize => "File Size",
            Self::BlockSize => "Block Size",
            Self::Duration => "Duration",
            Self::Threads => "Threads",
        }
    }
}

/// Configuration screen component
pub struct ConfigScreen {
    config: BenchmarkConfig,
    fields: Vec<ConfigField>,
    selected_field_index: usize,
    dropdown_state: ListState,
    is_dropdown_active: bool,
}

impl ConfigScreen {
    /// Create a new config screen
    pub fn new(config: &BenchmarkConfig) -> Self {
        Self {
            config: config.clone(),
            fields: ConfigField::all(),
            selected_field_index: 0,
            dropdown_state: ListState::default(),
            is_dropdown_active: false,
        }
    }

    /// Get the updated configuration
    pub fn get_config(&self) -> BenchmarkConfig {
        self.config.clone()
    }

    /// Handle key events for the config screen
    pub fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Option<AppState> {
        if self.is_dropdown_active {
            self.handle_dropdown_events(key);
        } else {
            match key.code {
                crossterm::event::KeyCode::Up => self.select_previous_field(),
                crossterm::event::KeyCode::Down => self.select_next_field(),
                crossterm::event::KeyCode::Enter => self.is_dropdown_active = true,
                crossterm::event::KeyCode::Esc => return Some(AppState::Start),
                _ => {}
            }
        }
        None
    }

    fn handle_dropdown_events(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            crossterm::event::KeyCode::Up => self.select_previous_option(),
            crossterm::event::KeyCode::Down => self.select_next_option(),
            crossterm::event::KeyCode::Enter => self.confirm_selection(),
            crossterm::event::KeyCode::Esc => self.is_dropdown_active = false,
            _ => {}
        }
    }

    fn select_previous_field(&mut self) {
        if self.selected_field_index > 0 {
            self.selected_field_index -= 1;
        }
    }

    fn select_next_field(&mut self) {
        if self.selected_field_index < self.fields.len() - 1 {
            self.selected_field_index += 1;
        }
    }

    fn select_previous_option(&mut self) {
        let _options = self.get_current_options();
        let selected = self.dropdown_state.selected().unwrap_or(0);
        if selected > 0 {
            self.dropdown_state.select(Some(selected - 1));
        }
    }

    fn select_next_option(&mut self) {
        let options = self.get_current_options();
        let selected = self.dropdown_state.selected().unwrap_or(0);
        if selected < options.len() - 1 {
            self.dropdown_state.select(Some(selected + 1));
        }
    }

    fn confirm_selection(&mut self) {
        let selected_index = self.dropdown_state.selected().unwrap_or(0);
        let field = &self.fields[self.selected_field_index];

        match field {
            ConfigField::Disk => {
                let disks = self.get_disk_options();
                self.config.disk_path = PathBuf::from(disks[selected_index].clone());
            }
            ConfigField::Mode => {
                let modes = self.get_mode_options();
                self.config.mode = modes[selected_index].clone();
                // Update defaults when mode changes
                self.config.block_size = self.config.mode.default_block_size();
                self.config.thread_count = self.config.mode.default_thread_count();
            }
            ConfigField::FileSize => {
                let sizes = self.get_filesize_options();
                self.config.file_size = sizes[selected_index];
            }
            ConfigField::BlockSize => {
                let sizes = self.get_blocksize_options();
                self.config.block_size = sizes[selected_index];
            }
            ConfigField::Duration => {
                let durations = self.get_duration_options();
                self.config.duration = durations[selected_index];
            }
            ConfigField::Threads => {
                let threads = self.get_thread_options();
                self.config.thread_count = threads[selected_index];
            }
        }
        self.is_dropdown_active = false;
    }

    fn get_current_options(&self) -> Vec<String> {
        let field = &self.fields[self.selected_field_index];
        match field {
            ConfigField::Disk => self.get_disk_options(),
            ConfigField::Mode => self
                .get_mode_options()
                .iter()
                .map(|m| format!("{:?}", m))
                .collect(),
            ConfigField::FileSize => self
                .get_filesize_options()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            ConfigField::BlockSize => self
                .get_blocksize_options()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            ConfigField::Duration => self
                .get_duration_options()
                .iter()
                .map(|d| format_duration(*d))
                .collect(),
            ConfigField::Threads => self
                .get_thread_options()
                .iter()
                .map(|t| t.to_string())
                .collect(),
        }
    }

    fn get_disk_options(&self) -> Vec<String> {
        // In a real implementation, this would scan for disks.
        vec!["/dev/sda".to_string(), "C:\\".to_string()]
    }

    fn get_mode_options(&self) -> Vec<BenchmarkMode> {
        vec![
            BenchmarkMode::SequentialWrite,
            BenchmarkMode::SequentialRead,
            BenchmarkMode::RandomReadWrite,
            BenchmarkMode::Mixed { read_ratio: 0.7 },
        ]
    }

    fn get_filesize_options(&self) -> Vec<u64> {
        vec![
            1 * 1024 * 1024 * 1024,
            2 * 1024 * 1024 * 1024,
            4 * 1024 * 1024 * 1024,
            8 * 1024 * 1024 * 1024,
        ]
    }

    fn get_blocksize_options(&self) -> Vec<u64> {
        vec![512, 4 * 1024, 128 * 1024, 1 * 1024 * 1024]
    }

    fn get_duration_options(&self) -> Vec<Duration> {
        vec![
            Duration::from_secs(15),
            Duration::from_secs(30),
            Duration::from_secs(60),
            Duration::from_secs(120),
        ]
    }

    fn get_thread_options(&self) -> Vec<usize> {
        vec![1, 2, 4, 8, 16]
    }

    /// Render the config screen
    pub fn render(&mut self, area: Rect, frame: &mut Frame, _state: &mut ()) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Help text
            ])
            .split(area);

        self.render_title(frame, chunks[0]);
        self.render_fields(frame, chunks[1]);
        self.render_help(frame, chunks[2]);

        if self.is_dropdown_active {
            self.render_dropdown(frame, chunks[1]);
        }
    }

    fn render_title(&self, frame: &mut Frame, area: Rect) {
        let title = Paragraph::new("Benchmark Configuration")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(title, area);
    }

    fn render_fields(&self, frame: &mut Frame, area: Rect) {
        let constraints: Vec<Constraint> =
            self.fields.iter().map(|_| Constraint::Length(3)).collect();
        let field_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        for (i, field) in self.fields.iter().enumerate() {
            let style = if i == self.selected_field_index {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default()
            };
            let block = Block::default().borders(Borders::ALL).border_style(style);
            let text = format!("{}: {}", field.title(), self.get_field_value(field));
            let p = Paragraph::new(text).block(block);
            frame.render_widget(p, field_chunks[i]);
        }
    }

    fn get_field_value(&self, field: &ConfigField) -> String {
        match field {
            ConfigField::Disk => self.config.disk_path.to_string_lossy().to_string(),
            ConfigField::Mode => format!("{:?}", self.config.mode),
            ConfigField::FileSize => self.config.file_size.to_string(),
            ConfigField::BlockSize => self.config.block_size.to_string(),
            ConfigField::Duration => format_duration(self.config.duration),
            ConfigField::Threads => self.config.thread_count.to_string(),
        }
    }

    fn render_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = "↑↓: Navigate | Enter: Select | Esc: Back";
        let help = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(help, area);
    }

    fn render_dropdown(&mut self, frame: &mut Frame, area: Rect) {
        let options = self.get_current_options();
        let items: Vec<ListItem> = options.iter().map(|o| ListItem::new(o.as_str())).collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select an option"),
            )
            .highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black))
            .highlight_symbol(">> ");

        let list_height = (options.len() + 2).min(10) as u16;
        let list_area = centered_rect(50, list_height, area);

        frame.render_widget(Clear, list_area);
        frame.render_stateful_widget(list, list_area, &mut self.dropdown_state);
    }
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent};

    #[test]
    fn test_field_navigation() {
        let cfg = BenchmarkConfig::default();
        let mut screen = ConfigScreen::new(&cfg);
        assert_eq!(screen.selected_field_index, 0);
        screen.select_next_field();
        assert_eq!(screen.selected_field_index, 1);
        screen.select_previous_field();
        assert_eq!(screen.selected_field_index, 0);
    }

    #[test]
    fn test_mode_default_updates() {
        let cfg = BenchmarkConfig::default();
        let mut screen = ConfigScreen::new(&cfg);
        // select Mode field
        screen.selected_field_index = 1; // Mode
        screen.is_dropdown_active = true;
        screen.dropdown_state.select(Some(2)); // RandomReadWrite
        screen.handle_key_event(KeyEvent::from(KeyCode::Enter));
        assert!(matches!(
            screen.get_config().mode,
            BenchmarkMode::RandomReadWrite
        ));
        assert_eq!(
            screen.get_config().block_size,
            BenchmarkMode::RandomReadWrite.default_block_size()
        );
    }
}
