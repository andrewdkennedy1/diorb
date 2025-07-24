//! Running screen implementation
//!
//! Displays real-time benchmark progress with live metrics including
//! MB/s, IOPS, latency statistics, progress bar, and cancellation handling.

use crate::bench::worker::AggregatedProgress;
use crate::util::units::{format_bytes, format_duration, format_throughput};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table},
    Frame,
};

/// Running screen component that displays live benchmark metrics
#[derive(Debug, Default)]
pub struct RunningScreen {
    /// Current progress data from benchmark workers
    current_progress: Option<AggregatedProgress>,
    /// Whether cancellation has been requested
    cancellation_requested: bool,
    /// Error message if benchmark failed
    error_message: Option<String>,
}

impl RunningScreen {
    /// Create a new running screen
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the screen with new progress data
    pub fn update_progress(&mut self, progress: AggregatedProgress) {
        self.current_progress = Some(progress);
    }

    /// Set an error message
    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
    }

    /// Clear any existing error message
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Request cancellation
    pub fn request_cancellation(&mut self) {
        self.cancellation_requested = true;
    }

    /// Check if cancellation was requested
    pub fn is_cancellation_requested(&self) -> bool {
        self.cancellation_requested
    }

    /// Check if benchmark is completed
    pub fn is_completed(&self) -> bool {
        if let Some(progress) = &self.current_progress {
            progress.completion_percentage() >= 1.0
        } else {
            false
        }
    }

    /// Check if there's an error
    pub fn has_error(&self) -> bool {
        self.error_message.is_some()
    }

    /// Get the error message
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// Render the running screen
    pub fn render(&mut self, f: &mut Frame) {
        let size = f.size();

        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(5), // Progress bar
                Constraint::Min(8),    // Metrics table
                Constraint::Length(4), // Status/Error area
                Constraint::Length(3), // Help text
            ])
            .split(size);

        // Render title
        self.render_title(f, chunks[0]);

        // Render progress bar
        self.render_progress_bar(f, chunks[1]);

        // Render metrics
        self.render_metrics(f, chunks[2]);

        // Render status/error
        self.render_status(f, chunks[3]);

        // Render help
        self.render_help(f, chunks[4]);
    }

    /// Render the title section
    fn render_title(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let title = if self.cancellation_requested {
            "Benchmark - Cancelling..."
        } else if self.has_error() {
            "Benchmark - Error"
        } else if self.is_completed() {
            "Benchmark - Completed"
        } else {
            "Benchmark - Running"
        };

        let color = if self.has_error() {
            Color::Red
        } else if self.cancellation_requested {
            Color::Yellow
        } else if self.is_completed() {
            Color::Green
        } else {
            Color::Cyan
        };

        let title_widget = Paragraph::new(title)
            .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(color)),
            );

        f.render_widget(title_widget, area);
    }

    /// Render the progress bar
    fn render_progress_bar(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let (progress_ratio, progress_text) = if let Some(progress) = &self.current_progress {
            let ratio = progress.completion_percentage();
            let percentage = (ratio * 100.0) as u16;

            let elapsed_str = format_duration(progress.elapsed);
            let eta_str = if let Some(eta) = progress.eta {
                format!(" | ETA: {}", format_duration(eta))
            } else {
                String::new()
            };

            let text = format!("{}% | Elapsed: {}{}", percentage, elapsed_str, eta_str);
            (ratio, text)
        } else {
            (0.0, "Starting...".to_string())
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title("Progress")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .gauge_style(Style::default().fg(Color::Green))
            .percent((progress_ratio * 100.0) as u16)
            .label(progress_text);

        f.render_widget(gauge, area);
    }

    /// Render the metrics table
    fn render_metrics(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let (throughput_str, iops_str, data_processed_str, workers_str, elapsed_str) =
            if let Some(progress) = &self.current_progress {
                (
                    format_throughput(progress.avg_throughput_mbps),
                    format!("{:.0}", progress.total_iops),
                    format!(
                        "{} / {}",
                        format_bytes(progress.total_bytes_processed),
                        format_bytes(progress.total_bytes_target)
                    ),
                    format!("{}", progress.active_workers),
                    format_duration(progress.elapsed),
                )
            } else {
                (
                    "0 MB/s".to_string(),
                    "0".to_string(),
                    "0 B".to_string(),
                    "0".to_string(),
                    "0s".to_string(),
                )
            };

        let rows = if self.current_progress.is_some() {
            vec![
                Row::new(vec!["Throughput:", &throughput_str]),
                Row::new(vec!["IOPS:", &iops_str]),
                Row::new(vec!["Data Processed:", &data_processed_str]),
                Row::new(vec!["Active Workers:", &workers_str]),
                Row::new(vec!["Elapsed Time:", &elapsed_str]),
            ]
        } else {
            vec![
                Row::new(vec!["Status:", "Initializing..."]),
                Row::new(vec!["Throughput:", "0 MB/s"]),
                Row::new(vec!["IOPS:", "0"]),
                Row::new(vec!["Data Processed:", "0 B"]),
                Row::new(vec!["Active Workers:", "0"]),
            ]
        };

        let table = Table::new(rows, [Constraint::Length(20), Constraint::Min(20)])
            .block(
                Block::default()
                    .title("Live Metrics")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .column_spacing(2);

        f.render_widget(table, area);
    }

    /// Render status or error information
    fn render_status(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let (text, style) = if let Some(error) = &self.error_message {
            (
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Error occurred:",
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(error, Style::default().fg(Color::Red))),
                ],
                Style::default().fg(Color::Red),
            )
        } else if self.cancellation_requested {
            (
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Cancellation requested...",
                        Style::default().fg(Color::Yellow),
                    )),
                    Line::from("Please wait for cleanup to complete."),
                ],
                Style::default().fg(Color::Yellow),
            )
        } else if self.is_completed() {
            (
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "Benchmark completed successfully!",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from("Press Enter to view detailed results."),
                ],
                Style::default().fg(Color::Green),
            )
        } else {
            (
                vec![
                    Line::from(""),
                    Line::from("Benchmark is running..."),
                    Line::from("Press 'c' to cancel or Esc to go back."),
                ],
                Style::default().fg(Color::White),
            )
        };

        let status = Paragraph::new(text).alignment(Alignment::Center).block(
            Block::default()
                .title("Status")
                .borders(Borders::ALL)
                .border_style(style),
        );

        f.render_widget(status, area);
    }

    /// Render help text
    fn render_help(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let help_text = if self.is_completed() {
            vec![Line::from(vec![
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" View Results  "),
                Span::styled(
                    "Esc",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Back to Menu"),
            ])]
        } else if self.has_error() {
            vec![Line::from(vec![
                Span::styled(
                    "Esc",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Back to Menu  "),
                Span::styled(
                    "R",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Retry"),
            ])]
        } else {
            vec![Line::from(vec![
                Span::styled(
                    "C",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Cancel  "),
                Span::styled(
                    "Esc",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Back"),
            ])]
        };

        let help = Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );

        f.render_widget(help, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench::worker::AggregatedProgress;
    use std::time::Duration;

    fn sample_progress(done: bool) -> AggregatedProgress {
        AggregatedProgress {
            total_bytes_processed: if done { 100 } else { 50 },
            total_bytes_target: 100,
            avg_throughput_mbps: 10.0,
            total_iops: 100.0,
            elapsed: Duration::from_secs(1),
            eta: None,
            active_workers: 1,
            worker_progress: Vec::new(),
        }
    }

    #[test]
    fn test_completion_detection() {
        let mut screen = RunningScreen::new();
        assert!(!screen.is_completed());
        screen.update_progress(sample_progress(true));
        assert!(screen.is_completed());
    }
}
