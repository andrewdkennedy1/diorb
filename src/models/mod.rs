//! Data models module
//! 
//! Contains benchmark configuration structures, result data models,
//! and performance metrics definitions.

pub mod result;

// Re-export commonly used types
pub use result::{
    BenchmarkResult,
    PerformanceMetrics,
    LatencyStats,
    SystemInfo,
    StorageInfo,
    StorageType,
};