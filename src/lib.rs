//! DIORB - Disk IO Rust Bench
//!
//! A cross-platform TUI application for disk performance benchmarking
//! with real-time feedback and accurate measurements.

use std::fmt;

// Public re-exports
pub mod bench;
pub mod config;
pub mod io;
pub mod models;
pub mod simple;
pub mod util;

// Common error types
#[derive(Debug)]
pub enum DIOrbError {
    /// I/O operation failed
    IoError(std::io::Error),
    /// Configuration validation or parsing error
    ConfigError(String),
    /// Benchmark execution error
    BenchmarkError(String),
    /// TUI rendering or interaction error
    TuiError(String),
    /// Permission denied for disk operations
    PermissionDenied(String),
    /// Insufficient disk space
    InsufficientSpace(String),
    /// Direct I/O not supported on this platform/filesystem
    DirectIoUnsupported(String),
    /// Temporary file creation failed
    TempFileError(String),
    /// Results persistence error
    PersistenceError(String),
    /// Worker management error
    WorkerError(String),
    /// Cancellation error
    CancellationError(String),
}

impl fmt::Display for DIOrbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DIOrbError::IoError(err) => write!(f, "I/O error: {}", err),
            DIOrbError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            DIOrbError::BenchmarkError(msg) => write!(f, "Benchmark error: {}", msg),
            DIOrbError::TuiError(msg) => write!(f, "TUI error: {}", msg),
            DIOrbError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            DIOrbError::InsufficientSpace(msg) => write!(f, "Insufficient disk space: {}", msg),
            DIOrbError::DirectIoUnsupported(msg) => write!(f, "Direct I/O not supported: {}", msg),
            DIOrbError::TempFileError(msg) => write!(f, "Temporary file error: {}", msg),
            DIOrbError::PersistenceError(msg) => write!(f, "Results persistence error: {}", msg),
            DIOrbError::WorkerError(msg) => write!(f, "Worker error: {}", msg),
            DIOrbError::CancellationError(msg) => write!(f, "Cancellation error: {}", msg),
        }
    }
}

impl std::error::Error for DIOrbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DIOrbError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for DIOrbError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::PermissionDenied => {
                DIOrbError::PermissionDenied(format!("Access denied: {}", err))
            }
            std::io::ErrorKind::OutOfMemory => {
                DIOrbError::InsufficientSpace(format!("Out of memory: {}", err))
            }
            _ => DIOrbError::IoError(err),
        }
    }
}

impl From<serde_json::Error> for DIOrbError {
    fn from(err: serde_json::Error) -> Self {
        DIOrbError::PersistenceError(format!("JSON serialization error: {}", err))
    }
}

impl From<toml::de::Error> for DIOrbError {
    fn from(err: toml::de::Error) -> Self {
        DIOrbError::ConfigError(format!("TOML parsing error: {}", err))
    }
}

impl From<toml::ser::Error> for DIOrbError {
    fn from(err: toml::ser::Error) -> Self {
        DIOrbError::ConfigError(format!("TOML serialization error: {}", err))
    }
}

/// Result type alias for DIORB operations
pub type Result<T> = std::result::Result<T, DIOrbError>;

/// Error handling utilities
pub mod error {
    use super::{DIOrbError, Result};
    use std::time::Duration;
    use tokio::time::sleep;

    /// Retry configuration for transient operations
    #[derive(Debug, Clone)]
    pub struct RetryConfig {
        /// Maximum number of retry attempts
        pub max_attempts: usize,
        /// Initial delay between retries
        pub initial_delay: Duration,
        /// Multiplier for exponential backoff
        pub backoff_multiplier: f64,
        /// Maximum delay between retries
        pub max_delay: Duration,
    }

    impl Default for RetryConfig {
        fn default() -> Self {
            Self {
                max_attempts: 3,
                initial_delay: Duration::from_millis(100),
                backoff_multiplier: 2.0,
                max_delay: Duration::from_secs(5),
            }
        }
    }

    /// Retry a fallible async operation with exponential backoff
    pub async fn retry_async<F, Fut, T>(operation: F, config: RetryConfig) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut delay = config.initial_delay;
        let mut last_error = None;

        for attempt in 0..config.max_attempts {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    last_error = Some(err);

                    // Don't retry on certain error types
                    if let Some(ref error) = last_error {
                        if !is_retryable_error(error) {
                            break;
                        }
                    }

                    // Don't sleep after the last attempt
                    if attempt < config.max_attempts - 1 {
                        sleep(delay).await;
                        delay = std::cmp::min(
                            Duration::from_millis(
                                (delay.as_millis() as f64 * config.backoff_multiplier) as u64,
                            ),
                            config.max_delay,
                        );
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            DIOrbError::BenchmarkError("Retry failed with no error".to_string())
        }))
    }

    /// Check if an error is retryable
    pub fn is_retryable_error(error: &DIOrbError) -> bool {
        match error {
            // Retryable errors
            DIOrbError::IoError(io_err) => {
                matches!(
                    io_err.kind(),
                    std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::ConnectionReset
                )
            }
            DIOrbError::TempFileError(_) => true,
            DIOrbError::WorkerError(_) => true,

            // Non-retryable errors
            DIOrbError::PermissionDenied(_) => false,
            DIOrbError::ConfigError(_) => false,
            DIOrbError::DirectIoUnsupported(_) => false,
            DIOrbError::CancellationError(_) => false,

            // Other errors are retryable by default
            _ => true,
        }
    }

    /// Convert error to user-friendly message with suggestions
    pub fn user_friendly_message(error: &DIOrbError) -> String {
        match error {
            DIOrbError::PermissionDenied(_) => {
                "Permission denied. Try running as administrator or check file permissions."
                    .to_string()
            }
            DIOrbError::InsufficientSpace(_) => {
                "Insufficient disk space. Free up space or choose a smaller file size.".to_string()
            }
            DIOrbError::DirectIoUnsupported(_) => {
                "Direct I/O not supported on this filesystem. Results may be less accurate."
                    .to_string()
            }
            DIOrbError::TempFileError(_) => {
                "Failed to create temporary files. Check disk space and permissions.".to_string()
            }
            DIOrbError::ConfigError(msg) => {
                format!("Configuration error: {}. Check your settings.", msg)
            }
            DIOrbError::PersistenceError(_) => {
                "Failed to save results. Check disk space and permissions.".to_string()
            }
            DIOrbError::CancellationError(_) => "Operation was cancelled by user.".to_string(),
            _ => error.to_string(),
        }
    }

    /// Create fallback strategies for common errors
    pub fn create_fallback_strategy(error: &DIOrbError) -> Option<String> {
        match error {
            DIOrbError::DirectIoUnsupported(_) => Some(
                "Falling back to buffered I/O. Results may be less accurate but still useful."
                    .to_string(),
            ),
            DIOrbError::PermissionDenied(_) => Some(
                "Try selecting a different disk location or running with elevated privileges."
                    .to_string(),
            ),
            DIOrbError::InsufficientSpace(_) => Some(
                "Consider reducing the file size or selecting a different disk with more space."
                    .to_string(),
            ),
            _ => None,
        }
    }
}

// Common types and constants
pub const APP_NAME: &str = "diorb";
pub const CONFIG_FILE: &str = "diorb.toml";
pub const RESULTS_FILE: &str = "results.json";
pub const TEMP_FILE_PREFIX: &str = "DIORB_TMP_";
pub const MAX_RESULTS_HISTORY: usize = 100;
