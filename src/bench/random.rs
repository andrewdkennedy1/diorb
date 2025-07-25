//! Random and mixed benchmark operations
//!
//! Implements random read/write benchmarks with configurable read ratio
//! and duration-based execution.

use crate::bench::sequential::ProgressUpdate;
use crate::{
    config::BenchmarkConfig,
    io::buffer::BufferPool,
    io::disk::{DiskIO, PlatformDiskIO},
    models::{BenchmarkResult, LatencyStats, PerformanceMetrics},
    DIOrbError, Result,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::io::SeekFrom;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Random benchmark executor
pub struct RandomBenchmark {
    config: BenchmarkConfig,
    disk_io: PlatformDiskIO,
    buffer_pool: Arc<BufferPool>,
}

impl RandomBenchmark {
    /// Create a new random benchmark executor
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

    /// Execute the benchmark with given read_ratio (0.0 = all writes, 1.0 = all reads)
    pub async fn run(
        &self,
        read_ratio: f32,
        progress_tx: mpsc::Sender<ProgressUpdate>,
    ) -> Result<BenchmarkResult> {
        let start_time = Instant::now();

        // Create temp file and fill with pattern
        let mut temp_file = self
            .disk_io
            .create_temp_file(&self.config.disk_path, self.config.file_size)
            .map_err(|e| DIOrbError::TempFileError(e.to_string()))?;
        if self.config.keep_temp_files {
            temp_file.keep_on_drop();
        }

        // Fill file with data so reads are valid
        let mut buffer = self.buffer_pool.get_buffer().await?;
        let pattern = create_test_pattern(buffer.len());
        buffer.copy_from_slice(&pattern);
        let mut bytes_written = 0u64;
        while bytes_written < self.config.file_size {
            let remaining = self.config.file_size - bytes_written;
            let write_size = std::cmp::min(remaining, self.config.block_size);
            let write_buf = &buffer[..write_size as usize];
            temp_file
                .file
                .write_direct(write_buf)
                .map_err(|e| DIOrbError::BenchmarkError(format!("Write failed: {}", e)))?;
            bytes_written += write_size;
        }
        temp_file
            .file
            .sync_all()
            .map_err(|e| DIOrbError::BenchmarkError(format!("Sync failed: {}", e)))?;

        let mut read_file = self
            .disk_io
            .open_direct_read(temp_file.path())
            .map_err(|e| DIOrbError::BenchmarkError(format!("Open read failed: {}", e)))?;
        let mut write_file = self
            .disk_io
            .open_direct_write(temp_file.path())
            .map_err(|e| DIOrbError::BenchmarkError(format!("Open write failed: {}", e)))?;

        let mut rng = SmallRng::from_entropy();
        let mut bytes_processed = 0u64;
        let mut operations = 0u64;
        let mut latency_samples = Vec::new();
        let mut last_update = Instant::now();
        let duration_ns = self.config.duration.as_nanos();

        while start_time.elapsed() < self.config.duration {
            let offset = rng.gen_range(0..(self.config.file_size - self.config.block_size));
            let is_read = rng.gen::<f32>() < read_ratio;
            let op_start = Instant::now();
            if is_read {
                read_file
                    .seek_direct(SeekFrom::Start(offset))
                    .map_err(|e| DIOrbError::BenchmarkError(format!("Seek failed: {}", e)))?;
                read_file
                    .read_direct(&mut buffer[..self.config.block_size as usize])
                    .map_err(|e| DIOrbError::BenchmarkError(format!("Read failed: {}", e)))?;
            } else {
                write_file
                    .seek_direct(SeekFrom::Start(offset))
                    .map_err(|e| DIOrbError::BenchmarkError(format!("Seek failed: {}", e)))?;
                write_file
                    .write_direct(&buffer[..self.config.block_size as usize])
                    .map_err(|e| DIOrbError::BenchmarkError(format!("Write failed: {}", e)))?;
            }
            latency_samples.push(op_start.elapsed());
            bytes_processed += self.config.block_size;
            operations += 1;

            if last_update.elapsed() >= Duration::from_millis(200) {
                let elapsed = start_time.elapsed();
                let ratio = (elapsed.as_nanos() as f64 / duration_ns as f64).min(1.0);
                let update = ProgressUpdate {
                    bytes_processed: (ratio * 1000.0) as u64,
                    total_bytes: 1000,
                    throughput_mbps: (bytes_processed as f64)
                        / (1024.0 * 1024.0)
                        / elapsed.as_secs_f64(),
                    iops: operations as f64 / elapsed.as_secs_f64(),
                    elapsed,
                    eta: if ratio >= 1.0 {
                        Some(Duration::ZERO)
                    } else {
                        let remaining = self.config.duration - elapsed;
                        Some(remaining)
                    },
                };
                if progress_tx.send(update).await.is_err() {
                    return Err(DIOrbError::CancellationError(
                        "Receiver dropped".to_string(),
                    ));
                }
                last_update = Instant::now();
            }
        }

        let total_elapsed = start_time.elapsed();
        let latency = if !latency_samples.is_empty() {
            let mut sorted = latency_samples.clone();
            sorted.sort();
            let min = sorted[0];
            let max = sorted[sorted.len() - 1];
            let avg = Duration::from_nanos(
                (sorted.iter().map(|d| d.as_nanos()).sum::<u128>() / sorted.len() as u128) as u64,
            );
            let mut percentiles = std::collections::HashMap::new();
            percentiles.insert(50, sorted[sorted.len() * 50 / 100]);
            percentiles.insert(95, sorted[sorted.len() * 95 / 100]);
            percentiles.insert(99, sorted[sorted.len() * 99 / 100]);
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

        let metrics = PerformanceMetrics {
            bytes_processed,
            elapsed_time: total_elapsed,
            throughput_mbps: if total_elapsed.as_secs_f64() > 0.0 {
                (bytes_processed as f64) / (1024.0 * 1024.0) / total_elapsed.as_secs_f64()
            } else {
                0.0
            },
            iops: if total_elapsed.as_secs_f64() > 0.0 {
                operations as f64 / total_elapsed.as_secs_f64()
            } else {
                0.0
            },
            latency,
        };

        let final_update = ProgressUpdate {
            bytes_processed: 1000,
            total_bytes: 1000,
            throughput_mbps: metrics.throughput_mbps,
            iops: metrics.iops,
            elapsed: total_elapsed,
            eta: Some(Duration::ZERO),
        };
        let _ = progress_tx.send(final_update).await;

        Ok(BenchmarkResult::new(self.config.clone(), metrics))
    }
}

fn create_test_pattern(size: usize) -> Vec<u8> {
    let mut pattern = Vec::with_capacity(size);
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
    #[ignore]
    async fn test_random_benchmark_runs() {
        let temp_dir = tempdir().unwrap();
        let config = BenchmarkConfig::random_read_write()
            .with_disk_path(temp_dir.path().to_path_buf())
            .with_duration(Duration::from_millis(500))
            .with_file_size(512 * 1024); // 512 KB
        let bench = RandomBenchmark::new(config).unwrap();
        let (tx, mut rx) = mpsc::channel(100);
        let handle = tokio::spawn(async move { bench.run(0.5, tx).await });
        let mut updates = Vec::new();
        while let Some(u) = rx.recv().await {
            updates.push(u);
        }
        let result = handle.await.unwrap().unwrap();
        assert!(result.metrics.bytes_processed > 0);
        assert!(!updates.is_empty());
        assert_eq!(updates.last().unwrap().bytes_processed, 1000);
    }
}
