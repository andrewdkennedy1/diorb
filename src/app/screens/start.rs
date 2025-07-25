//! Start screen implementation
//!
//! Main menu with Start Test, View Results, Settings, Exit options.
//! Includes navigation highlighting and responsive layout.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::path::PathBuf;

/// Start screen component with disk selection
#[derive(Debug)]
pub struct StartScreen {
    disks: Vec<PathBuf>,
    selected_index: usize,
    list_state: ListState,
    show_config_hint: bool,
}

impl StartScreen {
    /// Create a new start screen
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        // For now, we'll mock the disk list.
        // In a real application, this would scan the system.
        let disks = vec![PathBuf::from("C:\\"), PathBuf::from("D:\\")];

        Self {
            disks,
            selected_index: 0,
            list_state,
            show_config_hint: true,
        }
    }

    /// Detect available disks on the system
    fn detect_disks() -> Vec<PathBuf> {
        let mut disks = Vec::new();
        
        #[cfg(windows)]
        {
            // Try to get logical drives using Windows API
            let drives_mask = unsafe { 
                extern "system" {
                    fn GetLogicalDrives() -> u32;
                }
                GetLogicalDrives()
            };
            
            // Check each drive letter
            for i in 0..26 {
                if (drives_mask & (1 << i)) != 0 {
                    let drive_letter = (b'A' + i as u8) as char;
                    let path = PathBuf::from(format!("{}:\\", drive_letter));
                    
                    // Verify the drive is accessible
                    if path.exists() {
                        // Try to get some basic info about the drive
                        if let Ok(metadata) = std::fs::metadata(&path) {
                            if metadata.is_dir() {
                                disks.push(path);
                            }
                        }
                    }
                }
            }
            
            // If API approach failed, fallback to simple check
            if disks.is_empty() {
                for drive in 'C'..='Z' {
                    let path = PathBuf::from(format!("{}:\\", drive));
                    if path.exists() {
                        disks.push(path);
                    }
                }
            }
        }
        
        #[cfg(unix)]
        {
            // On Unix systems, check common mount points
            let common_paths = [
                "/",
                "/home",
                "/tmp",
                "/var",
                "/mnt",
                "/media",
            ];
            
            for &path_str in &common_paths {
                let path = PathBuf::from(path_str);
                if path.exists() && path.is_dir() {
                    // Check if we can write to this location
                    if let Ok(temp_file) = std::fs::File::create(path.join(".diorb_test")) {
                        drop(temp_file);
                        let _ = std::fs::remove_file(path.join(".diorb_test"));
                        disks.push(path);
                    }
                }
            }
            
            // Also check for mounted filesystems in /proc/mounts
            if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
                for line in mounts.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let mount_point = PathBuf::from(parts[1]);
                        if mount_point.exists() && mount_point.is_dir() && !disks.contains(&mount_point) {
                            // Only include real filesystems, not virtual ones
                            if !parts[0].starts_with("/proc") && !parts[0].starts_with("/sys") && !parts[0].starts_with("/dev/pts") {
                                disks.push(mount_point);
                            }
                        }
                    }
                }
            }
        }
        
        // Always include current directory as an option
        if let Ok(current_dir) = std::env::current_dir() {
            if !disks.contains(&current_dir) {
                disks.insert(0, current_dir);
            }
        }
        
        // Fallback if nothing was found
        if disks.is_empty() {
            disks.push(PathBuf::from("."));
        }
        
        // Sort disks for consistent display
        disks.sort();
        disks
    }

    /// Get the currently selected disk
    pub fn selected_disk(&self) -> &PathBuf {
        &self.disks[self.selected_index]
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = self.disks.len() - 1;
        }
        self.list_state.select(Some(self.selected_index));
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.selected_index < self.disks.len() - 1 {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
        }
        self.list_state.select(Some(self.selected_index));
    }

    /// Render the start screen
    pub fn render(&mut self, f: &mut Frame) {
        let size = f.size();

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Title and subtitle
                Constraint::Min(12),   // Disk list area
                Constraint::Length(3), // Help text
            ])
            .split(size);

        // Render title
        self.render_title(f, chunks[0]);

        // Render menu
        self.render_menu(f, chunks[1]);

        // Render help
        self.render_help(f, chunks[2]);
    }

    /// Render the title section
    fn render_title(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let title_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Main title
                Constraint::Length(2), // Subtitle
            ])
            .split(area);

        // Main title
        let title = Paragraph::new("DIORB")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );
        f.render_widget(title, title_chunks[0]);

        // Subtitle
        let subtitle = Paragraph::new("Disk I/O Rust Benchmark")
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);
        f.render_widget(subtitle, title_chunks[1]);
    }

    /// Render the main menu
    fn render_menu(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        let items: Vec<ListItem> = self
            .disks
            .iter()
            .map(|disk| {
                let display_text = if disk.to_string_lossy().len() > 50 {
                    format!("...{}", &disk.to_string_lossy()[disk.to_string_lossy().len()-47..])
                } else {
                    disk.to_string_lossy().into_owned()
                };
                ListItem::new(display_text)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select a Disk"),
            )
            .highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black))
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    /// Render the help text
    fn render_help(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let help_text = vec![Line::from(vec![
            Span::styled(
                "↑↓",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Navigate  "),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Select  "),
            Span::styled(
                "→",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" History  "),
            Span::styled(
                "Q",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Quit"),
        ])];

        let help = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );

        f.render_widget(help, area);
    }

    /// Handle key events for the start screen
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> StartScreenAction {
        match key {
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                self.select_previous();
                StartScreenAction::None
            }
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                self.select_next();
                StartScreenAction::None
            }
            crossterm::event::KeyCode::Enter | crossterm::event::KeyCode::Char(' ') => StartScreenAction::StartTest,
            crossterm::event::KeyCode::Char('c') | crossterm::event::KeyCode::Char('C') => StartScreenAction::OpenConfig,
            crossterm::event::KeyCode::Esc => StartScreenAction::Quit,
            _ => StartScreenAction::None,
        }
    }
}

/// Actions that can be triggered from the start screen
#[derive(Debug, Clone, PartialEq)]
pub enum StartScreenAction {
    None,
    StartTest,
    OpenConfig,
    Quit,
}

impl Default for StartScreen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_screen_creation() {
        let screen = StartScreen::new();
        assert_eq!(screen.selected_index, 0);
        assert!(!screen.disks.is_empty());
    }

    #[test]
    fn test_menu_navigation() {
        let mut screen = StartScreen::new();

        // Test moving down
        screen.select_next();
        assert_eq!(screen.selected_index, 1);

        // Test wrapping to beginning
        screen.select_next();
        assert_eq!(screen.selected_index, 0);
    }

    #[test]
    fn test_menu_navigation_up() {
        let mut screen = StartScreen::new();

        // Test moving up from first item (should wrap to last)
        screen.select_previous();
        assert_eq!(screen.selected_index, 1);

        // Test moving up normally
        screen.select_previous();
        assert_eq!(screen.selected_index, 0);
    }
}
