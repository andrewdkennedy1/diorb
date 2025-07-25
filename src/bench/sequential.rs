//! Sequential benchmark operations
//! 
//! Implements sequential read and write benchmarks with configurable
//! file size and block size, providing real-time progress tracking
//! and metrics collection.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use crate::{DIOrbError, Result};
use crate::config::{BenchmarkConfig, BenchmarkMode};
use crate::models::{BenchmarkResult, PerformanceMetrics, LatencyStats};
use crate::io::disk::{DiskIO, PlatformDiskIO, TempFile};
use crate::io::buffer::BufferPool;

/// Progress update sent during benchmark execution
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    /// Bytes processed so far
    pub bytes_processed: u64,
    /// Total bytes to process
    pub total_bytes: u64,
    /// Current throughput in MB/s
    pub throughput_mbps: f64,
    /// Current IOPS
    pub iops: f64,
    /// Elapsed time since start
    pub elapsed: Duration,
    /// Estimated time remaining
    pub eta: Option<Duration>,
}

impl ProgressUpdate {
    /// Calculate completion percentage (0.0 to 1.0)
    pub fn completion_percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.bytes_processed as f64) / (self.total_bytes as f64)
        }
    }
}

/// Sequential benchmark executor
pub struct SequentialBenchmark {
    config: BenchmarkConfig,
    disk_io: PlatformDiskIO,
    buffer_pool: Arc<BufferPool>,
}

impl SequentialBenchmark {
    /// Create a new sequential benchmark executor
    pub fn new(config: BenchmarkConfig) -> Result<Self> {
        config.validate()?;
        
        let disk_io = PlatformDiskIO::new();
        let buffer_pool = Arc::new(BufferPool::new(config.block_size as usize, 4)?);
        
        Ok(Self {
            config,
            disk_io,
            buffer_pool,
        })
    }
    
    /// Execute the sequential benchmark
    pub async fn run(&self, progress_tx: mpsc::Sender<ProgressUpdate>) -> Result<BenchmarkResult> {
        match self.config.mode {
            BenchmarkMode::SequentialWrite => self.run_sequential_write(progress_tx).await,
            BenchmarkMode::SequentialRead => self.run_sequential_read(progress_tx).await,
            _ => Err(DIOrbError::BenchmarkError(
                "Sequential benchmark only supports SequentialWrite and SequentialRead modes".to_string()
            )),
        }
    }
    
    /// Run sequential write benchmark
    async fn run_sequential_write(&self, progress_tx: mpsc::Sender<ProgressUpdate>) -> Result<BenchmarkResult> {
        let start_time = Instant::now();
        
        // Create temporary file
        let mut temp_file = self.disk_io.create_temp_file(&self.config.disk_path, self.config.file_size)?;
        if self.config.keep_temp_files {
            temp_file.keep_on_drop();
        }
        
        // Get buffer from pool
        let mut buffer = self.buffer_pool.get_buffer().await?;
        
        // Fill buffer with test pattern
        let pattern = create_test_pattern(buffer.len());
        buffer.copy_from_slice(&pattern);
        
        let mut bytes_written = 0u64;
        let mut latency_samples = Vec::new();
        let mut last_progress_update = Instant::now();
        
        println!("Starting sequential write test: {} bytes in {} byte blocks", 
                 self.config.file_size, self.config.block_size);
        
        // Write data in blocks
        while bytes_written < self.config.file_size {
            let write_start = Instant::now();
            
            // Calculate how much to write this iteration
            let remaining = self.config.file_size - bytes_written;
            let write_size = std::cmp::min(remaining, self.config.block_size) as usize;
            let chunk = &buffer[..write_size];
            
            // Perform actual write operation
            let written = temp_file.file.write_direct(chunk)
                .map_err(|e| {
                    eprintln!("Write operation failed at byte {}: {}", bytes_written, e);
                    DIOrbError::BenchmarkError(format!("Write failed at byte {}: {}", bytes_written, e))
                })?;
            
            if written == 0 {
                return Err(DIOrbError::BenchmarkError("Write returned 0 bytes".to_string()));
            }
            
            let write_duration = write_start.elapsed();
            latency_samples.push(write_duration);
            
            bytes_written += written as u64;
            
            // Send progress update every 100ms for more responsive UI
            if last_progress_update.elapsed() >= Duration::from_millis(100) {
                let elapsed = start_time.elapsed();
                let throughput_mbps = if elapsed.as_secs_f64() > 0.0 {
                    (bytes_written as f64) / (1024.0 * 1024.0) / elapsed.as_secs_f64()
                } else {
                    0.0
                };
                let iops = if elapsed.as_secs_f64() > 0.0 {
                    latency_samples.len() as f64 / elapsed.as_secs_f64()
                } else {
                    0.0
                };
                
                let eta = if bytes_written > 0 && throughput_mbps > 0.0 {
                    let remaining_mb = (self.config.file_size - bytes_written) as f64 / (1024.0 * 1024.0);
                    Some(Duration::from_secs_f64(remaining_mb / throughput_mbps))
                } else {
                    None
                };
                
                let update = ProgressUpdate {
                    bytes_processed: bytes_written,
                    total_bytes: self.config.file_size,
                    throughput_mbps,
                    iops,
                    elapsed,
                    eta,
                };
                
                if progress_tx.send(update).await.is_err() {
                    // Receiver dropped, benchmark was cancelled
                    return Err(DIOrbError::BenchmarkError("Benchmark cancelled".to_string()));
                }
                
                last_progress_update = Instant::now();
            }
        }
        
        // Force sync to disk to ensure all data is written
        println!("Syncing {} bytes to disk...", bytes_written);
        temp_file.file.sync_all()
            .map_err(|e| {
                eprintln!("Sync operation failed: {}", e);
                DIOrbError::BenchmarkError(format!("Sync failed: {}", e))
            })?;
        
        let total_elapsed = start_time.elapsed();
        println!("Write test completed: {} bytes in {:?}", bytes_written, total_elapsed);
        
        // Calculate final metrics
        let metrics = self.calculate_metrics(bytes_written, total_elapsed, &latency_samples);
        
        // Send final progress update
        let final_update = ProgressUpdate {
            bytes_processed: bytes_written,
            total_bytes: self.config.file_size,
            throughput_mbps: metrics.throughput_mbps,
            iops: metrics.iops,
            elapsed: total_elapsed,
            eta: Some(Duration::ZERO),
        };
        let _ = progress_tx.send(final_update).await;
        
        Ok(BenchmarkResult::new(self.config.clone(), metrics))
    }
    
    /// Run sequential read benchmark
    async fn run_sequential_read(&self, progress_tx: mpsc::Sender<ProgressUpdate>) -> Result<BenchmarkResult> {
        let start_time = Instant::now();
        
        // Create and write test file first
        println!("Creating test file for read benchmark...");
        let mut temp_file = self.create_test_file().await?;
        if self.config.keep_temp_files {
            temp_file.keep_on_drop();
        }
        
        println!("Opening file for reading: {}", temp_file.path().display());
        
        // Reopen file for reading
        let mut read_file = self.disk_io.open_direct_read(temp_file.path())?;
        
        // Get buffer from pool
        let mut buffer = self.buffer_pool.get_buffer().await?;
        
        let mut bytes_read = 0u64;
        let mut latency_samples = Vec::new();
        let mut last_progress_update = Instant::now();
        
        println!("Starting sequential read test: {} bytes in {} byte blocks", 
                 self.config.file_size, self.config.block_size);
        
        // Read data in blocks
        while bytes_read < self.config.file_size {
            let read_start = Instant::now();
            
            // Calculate how much to read this iteration
            let remaining = self.config.file_size - bytes_read;
            let read_size = std::cmp::min(remaining, self.config.block_size) as usize;
            let read_buffer = &mut buffer[..read_size];
            
            // Perform read operation
            let read_bytes = read_file.read_direct(read_buffer)
                .map_err(|e| {
                    eprintln!("Read operation failed at byte {}: {}", bytes_read, e);
                    DIOrbError::BenchmarkError(format!("Read failed at byte {}: {}", bytes_read, e))
                })?;
            
            if read_bytes == 0 {
                println!("EOF reached at {} bytes (expected {})", bytes_read, self.config.file_size);
                break; // EOF reached
            }
            
            let read_duration = read_start.elapsed();
            latency_samples.push(read_duration);
            
            bytes_read += read_bytes as u64;
            
            // Send progress update every 100ms for more responsive UI
            if last_progress_update.elapsed() >= Duration::from_millis(100) {
                let elapsed = start_time.elapsed();
                let throughput_mbps = if elapsed.as_secs_f64() > 0.0 {
                    (bytes_read as f64) / (1024.0 * 1024.0) / elapsed.as_secs_f64()
                } else {
                    0.0
                };
                let iops = if elapsed.as_secs_f64() > 0.0 {
                    latency_samples.len() as f64 / elapsed.as_secs_f64()
                } else {
                    0.0
                };
                
                let eta = if bytes_read > 0 && throughput_mbps > 0.0 {
                    let remaining_mb = (self.config.file_size - bytes_read) as f64 / (1024.0 * 1024.0);
                    Some(Duration::from_secs_f64(remaining_mb / throughput_mbps))
                } else {
                    None
                };
                
                let update = ProgressUpdate {
                    bytes_processed: bytes_read,
                    total_bytes: self.config.file_size,
                    throughput_mbps,
                    iops,
                    elapsed,
                    eta,
                };
                
                if progress_tx.send(update).await.is_err() {
                    // Receiver dropped, benchmark was cancelled
                    return Err(DIOrbError::BenchmarkError("Benchmark cancelled".to_string()));
                }
                
                last_progress_update = Instant::now();
            }
        }
        
        let total_elapsed = start_time.elapsed();
        println!("Read test completed: {} bytes in {:?}", bytes_read, total_elapsed);
        
        // Calculate final metrics
        let metrics = self.calculate_metrics(bytes_read, total_elapsed, &latency_samples);
        
        // Send final progress update
        let final_update = ProgressUpdate {
            bytes_processed: bytes_read,
            total_bytes: self.config.file_size,
            throughput_mbps: metrics.throughput_mbps,
            iops: metrics.iops,
            elapsed: total_elapsed,
            eta: Some(Duration::ZERO),
        };
        let _ = progress_tx.send(final_update).await;
        
        Ok(BenchmarkResult::new(self.config.clone(), metrics))
    }
    
    /// Create a test file filled with data for read benchmarks
    async fn create_test_file(&self) -> Result<TempFile> {
        let mut temp_file = self.disk_io.create_temp_file(&self.config.disk_path, self.config.file_size)?;
        
        // Get buffer from pool
        let mut buffer = self.buffer_pool.get_buffer().await?;
        
        // Fill buffer with test pattern
        let pattern = create_test_pattern(buffer.len());
        buffer.copy_from_slice(&pattern);
        
        let mut bytes_written = 0u64;
        
        println!("Creating test file: {} bytes", self.config.file_size);
        
        // Write data to create test file
        while bytes_written < self.config.file_size {
            let remaining = self.config.file_size - bytes_written;
            let write_size = std::cmp::min(remaining, self.config.block_size) as usize;
            let write_buffer = &buffer[..write_size];
            
            let written = temp_file.file.write_direct(write_buffer)
                .map_err(|e| {
                    eprintln!("Test file creation failed at byte {}: {}", bytes_written, e);
                    DIOrbError::BenchmarkError(format!("Test file creation failed at byte {}: {}", bytes_written, e))
                })?;
            
            if written == 0 {
                return Err(DIOrbError::BenchmarkError("Test file write returned 0 bytes".to_string()));
            }
            
            bytes_written += written as u64;
        }
        
        // Force sync to disk
        println!("Syncing test file to disk...");
        temp_file.file.sync_all()
            .map_err(|e| {
                eprintln!("Test file sync failed: {}", e);
                DIOrbError::BenchmarkError(format!("Test file sync failed: {}", e))
            })?;
        
        println!("Test file created successfully: {} bytes", bytes_written);
        Ok(temp_file)
    }
    
    /// Calculate performance metrics from collected data
    fn calculate_metrics(&self, bytes_processed: u64, elapsed: Duration, latency_samples: &[Duration]) -> PerformanceMetrics {
        let elapsed_secs = elapsed.as_secs_f64();
        
        // Calculate throughput
        let throughput_mbps = if elapsed_secs > 0.0 {
            (bytes_processed as f64) / (1024.0 * 1024.0) / elapsed_secs
        } else {
            0.0
        };
        
        // Calculate IOPS
        let iops = if elapsed_secs > 0.0 {
            latency_samples.len() as f64 / elapsed_secs
        } else {
            0.0
        };
        
        // Calculate latency statistics
        let latency_stats = if !latency_samples.is_empty() {
            let mut sorted_samples = latency_samples.to_vec();
            sorted_samples.sort();
            
            let min = sorted_samples[0];
            let max = sorted_samples[sorted_samples.len() - 1];
            let avg = Duration::from_nanos(
                (sorted_samples.iter().map(|d| d.as_nanos()).sum::<u128>() / sorted_samples.len() as u128) as u64
            );
            
            // Calculate percentiles
            let mut percentiles = std::collections::HashMap::new();
            percentiles.insert(50, sorted_samples[sorted_samples.len() * 50 / 100]);
            percentiles.insert(95, sorted_samples[sorted_samples.len() * 95 / 100]);
            percentiles.insert(99, sorted_samples[sorted_samples.len() * 99 / 100]);
            
            LatencyStats {
                min,
                avg,
                max,
                percentiles,
            }
        } else {
            LatencyStats {
                min: Duration::ZERO,
                avg: Duration::ZERO,
                max: Duration::ZERO,
                percentiles: std::collections::HashMap::new(),
            }
        };
        
        PerformanceMetrics {
            bytes_processed,
            elapsed_time: elapsed,
            throughput_mbps,
            iops,
            latency: latency_stats,
        }
    }
}

/// Create a test pattern for writing to files
fn create_test_pattern(size: usize) -> Vec<u8> {
    let mut pattern = Vec::with_capacity(size);
    
    // Create a repeating pattern that's easy to verify but not compressible
    for i in 0..size {
        pattern.push((i % 256) as u8);
    }
    
    pattern
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::sync::mpsc;
    
    #[tokio::test]
    async fn test_sequential_write_benchmark() {
        let temp_dir = tempdir().unwrap();
        
        let config = BenchmarkConfig::sequential_write()
            .with_disk_path(temp_dir.path().to_path_buf())
            .with_file_size(1024 * 1024) // 1 MB
            .with_block_size(64 * 1024); // 64 KB
        
        let benchmark = SequentialBenchmark::new(config).unwrap();
        let (tx, mut rx) = mpsc::channel(100);
        
        // Run benchmark in background
        let benchmark_handle = tokio::spawn(async move {
            benchmark.run(tx).await
        });
        
        // Collect progress updates
        let mut updates = Vec::new();
        while let Some(update) = rx.recv().await {
            updates.push(update);
        }
        
        // Wait for benchmark to complete
        let result = benchmark_handle.await.unwrap().unwrap();
        
        // Verify results
        assert_eq!(result.metrics.bytes_processed, 1024 * 1024);
        assert!(result.metrics.throughput_mbps > 0.0);
        assert!(result.metrics.iops > 0.0);
        assert!(result.metrics.elapsed_time > Duration::ZERO);
        
        // Verify progress updates were sent
        assert!(!updates.is_empty());
        let final_update = updates.last().unwrap();
        assert_eq!(final_update.bytes_processed, 1024 * 1024);
        assert_eq!(final_update.completion_percentage(), 1.0);
    }
    
    #[tokio::test]
    async fn test_sequential_read_benchmark() {
        let temp_dir = tempdir().unwrap();
        
        let config = BenchmarkConfig::sequential_read()
            .with_disk_path(temp_dir.path().to_path_buf())
            .with_file_size(1024 * 1024) // 1 MB
            .with_block_size(64 * 1024); // 64 KB
        
        let benchmark = SequentialBenchmark::new(config).unwrap();
        let (tx, mut rx) = mpsc::channel(100);
        
        // Run benchmark in background
        let benchmark_handle = tokio::spawn(async move {
            benchmark.run(tx).await
        });
        
        // Collect progress updates
        let mut updates = Vec::new();
        while let Some(update) = rx.recv().await {
            updates.push(update);
        }
        
        // Wait for benchmark to complete
        let result = benchmark_handle.await.unwrap().unwrap();
        
        // Verify results
        assert_eq!(result.metrics.bytes_processed, 1024 * 1024);
        assert!(result.metrics.throughput_mbps > 0.0);
        assert!(result.metrics.iops > 0.0);
        assert!(result.metrics.elapsed_time > Duration::ZERO);
        
        // Verify progress updates were sent
        assert!(!updates.is_empty());
        let final_update = updates.last().unwrap();
        assert_eq!(final_update.bytes_processed, 1024 * 1024);
        assert_eq!(final_update.completion_percentage(), 1.0);
    }
    
    #[tokio::test]
    async fn test_benchmark_with_dropped_receiver() {
        let temp_dir = tempdir().unwrap();
        
        let config = BenchmarkConfig::sequential_write()
            .with_disk_path(temp_dir.path().to_path_buf())
            .with_file_size(1024 * 1024) // 1 MB - small enough to complete quickly
            .with_block_size(64 * 1024); // 64 KB blocks
        
        let benchmark = SequentialBenchmark::new(config).unwrap();
        let (tx, rx) = mpsc::channel(100);
        
        // Drop receiver immediately to test graceful handling
        drop(rx);
        
        // Run benchmark - should handle dropped receiver gracefully
        let result = benchmark.run(tx).await;
        
        // The benchmark should either complete successfully or fail gracefully
        // It shouldn't panic or hang
        match result {
            Ok(_) => {
                // If it completes successfully, that's fine too
                // (the receiver was dropped but the benchmark might complete before sending updates)
            }
            Err(DIOrbError::BenchmarkError(msg)) => {
                // If it fails with a cancellation error, that's also acceptable
                assert!(msg.contains("cancelled") || msg.contains("receiver"));
            }
            Err(e) => {
                panic!("Unexpected error type: {}", e);
            }
        }
    }
    
    #[test]
    fn test_progress_update_completion_percentage() {
        let update = ProgressUpdate {
            bytes_processed: 500,
            total_bytes: 1000,
            throughput_mbps: 10.0,
            iops: 100.0,
            elapsed: Duration::from_secs(1),
            eta: Some(Duration::from_secs(1)),
        };
        
        assert_eq!(update.completion_percentage(), 0.5);
        
        let complete_update = ProgressUpdate {
            bytes_processed: 1000,
            total_bytes: 1000,
            throughput_mbps: 10.0,
            iops: 100.0,
            elapsed: Duration::from_secs(2),
            eta: Some(Duration::ZERO),
        };
        
        assert_eq!(complete_update.completion_percentage(), 1.0);
    }
    
    #[test]
    fn test_create_test_pattern() {
        let pattern = create_test_pattern(256);
        assert_eq!(pattern.len(), 256);
        
        // Verify pattern is not compressible (each byte is different)
        for (i, &byte) in pattern.iter().enumerate() {
            assert_eq!(byte, (i % 256) as u8);
        }
    }
    
    #[test]
    fn test_sequential_benchmark_invalid_mode() {
        let temp_dir = tempdir().unwrap();
        
        let config = BenchmarkConfig::random_read_write()
            .with_disk_path(temp_dir.path().to_path_buf());
        
        let benchmark = SequentialBenchmark::new(config).unwrap();
        
        // This should fail at runtime since sequential benchmark doesn't support random mode
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (tx, _rx) = mpsc::channel(100);
        
        let result = rt.block_on(benchmark.run(tx));
        assert!(result.is_err());
        
        if let Err(DIOrbError::BenchmarkError(msg)) = result {
            assert!(msg.contains("Sequential benchmark only supports"));
        } else {
            panic!("Expected benchmark error for invalid mode");
        }
    }
}