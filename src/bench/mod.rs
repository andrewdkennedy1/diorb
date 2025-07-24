//! Benchmark engine module
//!
//! Contains the core benchmarking logic, worker management,
//! and different benchmark mode implementations.

pub mod random;
pub mod sequential;
pub mod worker;

// Re-export commonly used types
pub use random::RandomBenchmark;
pub use sequential::{ProgressUpdate, SequentialBenchmark};
pub use worker::{AggregatedProgress, WorkerInfo, WorkerManager, WorkerStatus};
