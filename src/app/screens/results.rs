//! Results screen implementation
//! 
//! Displays comprehensive benchmark results with performance metrics,
//! system information, and options to save or return to menu.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Frame,
};
use crate::models::BenchmarkResult;
use crate::util::units::{format_bytes, format_duration, format_throughput};

/// Results screen component that displays benchmark results
#[derive(Debug)]
pub struct ResultsScreen {
    /// The benchmark result to display
    result: Option<BenchmarkResult>,
    /// Whether save operation is in progress
    saving: bool,
    /// Save operation result message
    save_message: Option<String>,
    /// Selected action (Save or Back)
    selected_action: ResultAction,
}

/// Available actions on the results screen
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultAction {
    Save,
    Back,
}

impl ResultAction {
    /// Get all available actions
    pub fn all() -> Vec<Self> {
        vec![Self::Save, Self::Back]
    }

    /// Get display text for the action
    pub fn display_text(&self) -> &'static str {
        match self {
            Self::Save => "Save Results",
            Self::Back => "Back to Menu",
        }
    }
}

impl ResultsScreen {
    /// Create a new results screen
    pub fn new() -> Self {
        Self {
            result: None,
            saving: false,
            save_message: None,
            selected_action: ResultAction::Save,
        }
    }

    /// Set the benchmark result to display
    pub fn set_result(&mut self, result: BenchmarkResult) {
        self.result = Some(result);
        self.save_message = None;
    }

    /// Get the current result
    pub fn result(&self) -> Option<&BenchmarkResult> {
        self.result.as_ref()
    }

    /// Start save operation
    pub fn start_save(&mut self) {
        self.saving = true;
        self.save_message = None;
    }

    /// Complete save operation with result
    pub fn complete_save(&mut self, _success: bool, message: String) {
        self.saving = false;
        self.save_message = Some(message);
    }

    /// Check if save operation is in progress
    pub fn is_saving(&self) -> bool {
        self.saving
    }

    /// Get save message
    pub fn save_message(&self) -> Option<&str> {
        self.save_message.as_deref()
    }

    /// Get selected action
    pub fn selected_action(&self) -> &ResultAction {
        &self.selected_action
    }

    /// Select next action
    pub fn select_next_action(&mut self) {
        let actions = ResultAction::all();
        let current_index = actions.iter().position(|a| a == &self.selected_action).unwrap_or(0);
        let next_index = (current_index + 1) % actions.len();
        self.selected_action = actions[next_index].clone();
    }

    /// Select previous action
    pub fn select_previous_action(&mut self) {
        let actions = ResultAction::all();
        let current_index = actions.iter().position(|a| a == &self.selected_action).unwrap_or(0);
        let prev_index = if current_index == 0 { actions.len() - 1 } else { current_index - 1 };
        self.selected_action = actions[prev_index].clone();
    }

    /// Render the results screen
    pub fn render(&mut self, f: &mut Frame) {
        let size = f.size();

        if self.result.is_none() {
            self.render_no_results(f, size);
            return;
        }

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(12),    // Results table
                Constraint::Length(4),  // Actions/Status
                Constraint::Length(3),  // Help text
            ])
            .split(size);

        // Render title
        self.render_title(f, chunks[0]);

        // Render results
        self.render_results_table(f, chunks[1]);

        // Render actions
        self.render_actions(f, chunks[2]);

        // Render help
        self.render_help(f, chunks[3]);
    }

    /// Render when no results are available
    fn render_no_results(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let text = vec![
            Line::from(""),
            Line::from("No benchmark results available"),
            Line::from(""),
            Line::from("Run a benchmark first to see results here."),
            Line::from(""),
            Line::from(Span::styled("Press Esc to go back", Style::default().fg(Color::Yellow))),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(Block::default()
                .title("Results")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)));

        f.render_widget(paragraph, area);
    }

    /// Render the title section
    fn render_title(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let title = Paragraph::new("Benchmark Results")
            .style(Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)));

        f.render_widget(title, area);
    }

    /// Render the results table
    fn render_results_table(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let result = self.result.as_ref().unwrap();
        let metrics = &result.metrics;

        // Pre-format all strings to avoid borrow checker issues
        let mode_str = format!("{:?}", result.config.mode);
        let file_size_str = format_bytes(result.config.file_size);
        let block_size_str = format_bytes(result.config.block_size);
        let threads_str = format!("{}", result.config.thread_count);
        let data_processed_str = format_bytes(metrics.bytes_processed);
        let elapsed_time_str = format_duration(metrics.elapsed_time);
        let throughput_str = format_throughput(metrics.throughput_mbps);
        let iops_str = format!("{:.0}", metrics.iops);
        let min_latency_str = format_duration(metrics.latency.min);
        let avg_latency_str = format_duration(metrics.latency.avg);
        let max_latency_str = format_duration(metrics.latency.max);
        let timestamp_str = result.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let disk_path_str = result.config.disk_path.display().to_string();

        // Pre-format percentile strings
        let p50_str = metrics.latency.percentiles.get(&50).map(|p| format_duration(*p));
        let p95_str = metrics.latency.percentiles.get(&95).map(|p| format_duration(*p));
        let p99_str = metrics.latency.percentiles.get(&99).map(|p| format_duration(*p));

        let mut final_rows = vec![
            Row::new(vec!["Test Configuration", ""]).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Row::new(vec!["  Mode:", mode_str.as_str()]),
            Row::new(vec!["  File Size:", file_size_str.as_str()]),
            Row::new(vec!["  Block Size:", block_size_str.as_str()]),
            Row::new(vec!["  Threads:", threads_str.as_str()]),
            Row::new(vec!["", ""]), // Spacer
            Row::new(vec!["Performance Results", ""]).style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Row::new(vec!["  Data Processed:", data_processed_str.as_str()]),
            Row::new(vec!["  Elapsed Time:", elapsed_time_str.as_str()]),
            Row::new(vec!["  Throughput:", throughput_str.as_str()]),
            Row::new(vec!["  IOPS:", iops_str.as_str()]),
            Row::new(vec!["", ""]), // Spacer
            Row::new(vec!["Latency Statistics", ""]).style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Row::new(vec!["  Minimum:", min_latency_str.as_str()]),
            Row::new(vec!["  Average:", avg_latency_str.as_str()]),
            Row::new(vec!["  Maximum:", max_latency_str.as_str()]),
        ];

        // Add percentile data if available
        if let Some(ref p50) = p50_str {
            final_rows.push(Row::new(vec!["  50th Percentile:", p50.as_str()]));
        }
        if let Some(ref p95) = p95_str {
            final_rows.push(Row::new(vec!["  95th Percentile:", p95.as_str()]));
        }
        if let Some(ref p99) = p99_str {
            final_rows.push(Row::new(vec!["  99th Percentile:", p99.as_str()]));
        }

        // Add timestamp and system info
        final_rows.push(Row::new(vec!["", ""])); // Spacer
        final_rows.push(Row::new(vec!["Test Information", ""]).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
        final_rows.push(Row::new(vec!["  Timestamp:", timestamp_str.as_str()]));
        let system_info = &result.system_info;
        final_rows.push(Row::new(vec!["  OS:", system_info.os.as_str()]));
        final_rows.push(Row::new(vec!["  CPU:", system_info.cpu.as_str()]));
        final_rows.push(Row::new(vec!["  Disk Path:", disk_path_str.as_str()]));

        let table = Table::new(
            final_rows,
            [Constraint::Length(20), Constraint::Min(30)]
        )
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)))
            .column_spacing(2);

        f.render_widget(table, area);
    }

    /// Render action buttons
    fn render_actions(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let actions_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(40), // Fixed width for actions
                Constraint::Min(0),
            ])
            .split(area)[1];

        let action_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(actions_area);

        // Render Save button
        let save_style = if self.selected_action == ResultAction::Save {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let save_text = if self.saving {
            "Saving..."
        } else {
            "Save Results"
        };

        let save_button = Paragraph::new(save_text)
            .style(save_style)
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(if self.selected_action == ResultAction::Save {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                }));

        f.render_widget(save_button, action_chunks[0]);

        // Render Back button
        let back_style = if self.selected_action == ResultAction::Back {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let back_button = Paragraph::new("Back to Menu")
            .style(back_style)
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(if self.selected_action == ResultAction::Back {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                }));

        f.render_widget(back_button, action_chunks[1]);

        // Show save message if available
        if let Some(message) = &self.save_message {
            let message_area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),
                    Constraint::Length(1),
                ])
                .split(area)[1];

            let message_style = if message.contains("Error") || message.contains("Failed") {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            };

            let message_widget = Paragraph::new(message.as_str())
                .style(message_style)
                .alignment(Alignment::Center);

            f.render_widget(message_widget, message_area);
        }
    }

    /// Render help text
    fn render_help(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let help_text = vec![
            Line::from(vec![
                Span::styled("←→", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" Navigate  "),
                Span::styled("Enter", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" Select  "),
                Span::styled("Esc", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" Back"),
            ]),
        ];

        let help = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)));

        f.render_widget(help, area);
    }
}

impl Default for ResultsScreen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BenchmarkConfig, BenchmarkMode};
    use crate::models::{LatencyStats, PerformanceMetrics, SystemInfo};
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;

    fn create_test_result() -> BenchmarkResult {
        let mut percentiles = HashMap::new();
        percentiles.insert(50, Duration::from_millis(5));
        percentiles.insert(95, Duration::from_millis(15));
        percentiles.insert(99, Duration::from_millis(25));

        BenchmarkResult {
            timestamp: Utc::now(),
            config: BenchmarkConfig {
                disk_path: PathBuf::from("/tmp/test"),
                mode: BenchmarkMode::SequentialWrite,
                file_size: 1024 * 1024 * 1024, // 1 GB
                block_size: 64 * 1024,         // 64 KB
                duration: Duration::from_secs(30),
                thread_count: 2,
                keep_temp_files: false,
            },
            metrics: PerformanceMetrics {
                bytes_processed: 1024 * 1024 * 1024,
                elapsed_time: Duration::from_secs(10),
                throughput_mbps: 100.0,
                iops: 1600.0,
                latency: LatencyStats {
                    min: Duration::from_millis(1),
                    avg: Duration::from_millis(8),
                    max: Duration::from_millis(30),
                    percentiles,
                },
            },
            system_info: SystemInfo {
                os: "Linux".to_string(),
                cpu: "Test CPU".to_string(),
                memory_total: 8 * 1024 * 1024 * 1024, // 8 GB
                memory_available: 4 * 1024 * 1024 * 1024,
                storage_info: Default::default(),
            },
        }
    }

    #[test]
    fn test_results_screen_creation() {
        let screen = ResultsScreen::new();
        assert!(screen.result.is_none());
        assert!(!screen.saving);
        assert!(screen.save_message.is_none());
        assert_eq!(screen.selected_action, ResultAction::Save);
    }

    #[test]
    fn test_set_result() {
        let mut screen = ResultsScreen::new();
        let result = create_test_result();
        
        screen.set_result(result.clone());
        assert!(screen.result.is_some());
        assert_eq!(screen.result().unwrap().metrics.bytes_processed, result.metrics.bytes_processed);
    }

    #[test]
    fn test_save_operations() {
        let mut screen = ResultsScreen::new();
        
        assert!(!screen.is_saving());
        assert!(screen.save_message().is_none());
        
        screen.start_save();
        assert!(screen.is_saving());
        
        screen.complete_save(true, "Results saved successfully".to_string());
        assert!(!screen.is_saving());
        assert_eq!(screen.save_message(), Some("Results saved successfully"));
        
        screen.complete_save(false, "Error saving results".to_string());
        assert_eq!(screen.save_message(), Some("Error saving results"));
    }

    #[test]
    fn test_action_navigation() {
        let mut screen = ResultsScreen::new();
        
        assert_eq!(screen.selected_action(), &ResultAction::Save);
        
        screen.select_next_action();
        assert_eq!(screen.selected_action(), &ResultAction::Back);
        
        screen.select_next_action();
        assert_eq!(screen.selected_action(), &ResultAction::Save); // Wraps around
        
        screen.select_previous_action();
        assert_eq!(screen.selected_action(), &ResultAction::Back);
        
        screen.select_previous_action();
        assert_eq!(screen.selected_action(), &ResultAction::Save); // Wraps around
    }

    #[test]
    fn test_result_actions() {
        let actions = ResultAction::all();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0], ResultAction::Save);
        assert_eq!(actions[1], ResultAction::Back);
        
        assert_eq!(ResultAction::Save.display_text(), "Save Results");
        assert_eq!(ResultAction::Back.display_text(), "Back to Menu");
    }
}