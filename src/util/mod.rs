//! Utility functions module
//! 
//! Contains helper functions for units formatting, duration parsing,
//! and other common operations.

pub mod units;

// Re-export commonly used functions
pub use units::{
    format_bytes, parse_bytes,
    format_duration, parse_duration,
    calculate_throughput_mbps, calculate_iops,
    format_throughput, format_iops, format_latency,
};