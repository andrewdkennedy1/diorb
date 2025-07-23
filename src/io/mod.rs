//! I/O operations module
//! 
//! Contains platform-specific disk I/O operations and abstractions
//! for cross-platform compatibility.

pub mod disk;
pub mod buffer;
pub mod async_ops;

#[cfg(test)]
mod integration_tests;

pub use disk::{DiskIO, DirectFile, TempFile, create_disk_io};
pub use buffer::{BufferPool, PooledBuffer};
pub use async_ops::{AsyncDiskIO, IOMetrics, StorageType, detect_storage_type};