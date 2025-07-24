//! History screen implementation
//!
//! Displays a list of past benchmark results loaded from persistence
//! and allows selecting a result for detailed view.

use crate::models::BenchmarkResult;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// History screen component
#[derive(Debug)]
pub struct HistoryScreen {
    results: Vec<BenchmarkResult>,
    selected_index: usize,
    list_state: ListState,
}

impl HistoryScreen {
    /// Create a new history screen from a list of results
    pub fn new(results: Vec<BenchmarkResult>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            results,
            selected_index: 0,
            list_state,
        }
    }

    /// Update results list
    pub fn set_results(&mut self, results: Vec<BenchmarkResult>) {
        self.results = results;
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    /// Get the currently selected result
    pub fn selected_result(&self) -> Option<&BenchmarkResult> {
        self.results.get(self.selected_index)
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.results.is_empty() {
            return;
        }
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = self.results.len() - 1;
        }
        self.list_state.select(Some(self.selected_index));
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.results.is_empty() {
            return;
        }
        if self.selected_index < self.results.len() - 1 {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
        }
        self.list_state.select(Some(self.selected_index));
    }

    /// Render the history screen
    pub fn render(&mut self, f: &mut Frame) {
        let size = f.size();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(size);

        self.render_title(f, chunks[0]);
        self.render_list(f, chunks[1]);
        self.render_help(f, chunks[2]);
    }

    fn render_title(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let title = Paragraph::new("Results History")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    fn render_list(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        let items: Vec<ListItem> = if self.results.is_empty() {
            vec![ListItem::new("No saved results")]
        } else {
            self.results
                .iter()
                .map(|r| ListItem::new(r.summary()))
                .collect()
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black))
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_help(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let text = Paragraph::new("↑↓: Navigate  Enter: Select  Esc: Back")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(text, area);
    }
}

impl Default for HistoryScreen {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BenchmarkConfig, BenchmarkMode};
    use crate::models::{BenchmarkResult, LatencyStats, PerformanceMetrics, SystemInfo};
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;

    fn create_result() -> BenchmarkResult {
        let mut percentiles = HashMap::new();
        percentiles.insert(50, Duration::from_millis(5));
        BenchmarkResult {
            timestamp: Utc::now(),
            config: BenchmarkConfig {
                disk_path: PathBuf::from("/tmp"),
                mode: BenchmarkMode::SequentialWrite,
                file_size: 1024,
                block_size: 512,
                duration: Duration::from_secs(1),
                thread_count: 1,
                keep_temp_files: false,
            },
            metrics: PerformanceMetrics {
                bytes_processed: 1024,
                elapsed_time: Duration::from_secs(1),
                throughput_mbps: 1.0,
                iops: 1000.0,
                latency: LatencyStats {
                    min: Duration::from_millis(1),
                    avg: Duration::from_millis(2),
                    max: Duration::from_millis(3),
                    percentiles,
                },
            },
            system_info: SystemInfo::default(),
        }
    }

    #[test]
    fn test_history_navigation() {
        let results = vec![create_result(), create_result()];
        let mut screen = HistoryScreen::new(results);
        assert_eq!(screen.selected_index, 0);
        screen.select_next();
        assert_eq!(screen.selected_index, 1);
        screen.select_next();
        assert_eq!(screen.selected_index, 0); // wrap
        screen.select_previous();
        assert_eq!(screen.selected_index, 1); // wrap
    }
}
