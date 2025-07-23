//! Benchmark engine module
//! 
//! Contains the core benchmarking logic, worker management,
//! and different benchmark mode implementations.

pub mod sequential;
pub mod worker;

// Re-export commonly used types
pub use sequential::{SequentialBenchmark, ProgressUpdate};
pub use worker::{WorkerManager, WorkerStatus, WorkerInfo, AggregatedProgress};