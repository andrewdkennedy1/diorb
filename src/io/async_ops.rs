use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task;

use super::buffer::{BufferPool, PooledBuffer};
use super::disk::{DiskIO, DirectFile, TempFile};

/// Async wrapper for disk I/O operations
pub struct AsyncDiskIO {
    inner: Arc<dyn DiskIO + Send + Sync>,
    buffer_pool: BufferPool,
}

impl AsyncDiskIO {
    pub fn new(disk_io: impl DiskIO + Send + Sync + 'static, block_size: usize) -> io::Result<Self> {
        Ok(Self {
            inner: Arc::new(disk_io),
            buffer_pool: BufferPool::new(block_size, 16)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?, // Pool up to 16 buffers
        })
    }
    
    /// Async write operation with buffer pooling
    pub async fn write_async(
        &self,
        mut file: Box<dyn DirectFile>,
        data: Vec<u8>,
    ) -> io::Result<(Box<dyn DirectFile>, usize, Duration)> {
        let start = Instant::now();
        
        let result = task::spawn_blocking(move || {
            let bytes_written = file.write_direct(&data)?;
            file.sync_all()?;
            Ok((file, bytes_written))
        }).await;
        
        let elapsed = start.elapsed();
        
        match result {
            Ok(Ok((file, bytes_written))) => Ok((file, bytes_written, elapsed)),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }
    
    /// Async read operation with buffer pooling
    pub async fn read_async(
        &self,
        mut file: Box<dyn DirectFile>,
        buffer_size: usize,
    ) -> io::Result<(Box<dyn DirectFile>, Vec<u8>, Duration)> {
        let start = Instant::now();
        
        let result = task::spawn_blocking(move || {
            let mut buffer = vec![0u8; buffer_size];
            let bytes_read = file.read_direct(&mut buffer)?;
            buffer.truncate(bytes_read);
            Ok((file, buffer))
        }).await;
        
        let elapsed = start.elapsed();
        
        match result {
            Ok(Ok((file, buffer))) => Ok((file, buffer, elapsed)),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }
    
    /// Create a temporary file asynchronously
    pub async fn create_temp_file_async(
        &self,
        target_dir: &Path,
        size_hint: u64,
    ) -> io::Result<TempFile> {
        let inner = Arc::clone(&self.inner);
        let target_dir = target_dir.to_owned();
        
        task::spawn_blocking(move || {
            inner.create_temp_file(&target_dir, size_hint)
        }).await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }
    
    /// Get optimal block size for path asynchronously
    pub async fn get_optimal_block_size_async(&self, path: &Path) -> io::Result<u64> {
        let inner = Arc::clone(&self.inner);
        let path = path.to_owned();
        
        task::spawn_blocking(move || {
            inner.get_optimal_block_size(&path)
        }).await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }
    
    /// Get a pooled buffer for I/O operations
    pub async fn get_pooled_buffer(&self) -> io::Result<PooledBuffer> {
        PooledBuffer::new(self.buffer_pool.clone()).await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }
    
    /// Get the buffer pool for advanced usage
    pub fn buffer_pool(&self) -> &BufferPool {
        &self.buffer_pool
    }
}

/// Performance metrics for I/O operations
#[derive(Debug, Clone)]
pub struct IOMetrics {
    pub bytes_processed: u64,
    pub elapsed_time: Duration,
    pub throughput_mbps: f64,
    pub iops: f64,
    pub operations_count: u64,
}

impl IOMetrics {
    pub fn new(bytes_processed: u64, elapsed_time: Duration, operations_count: u64) -> Self {
        let seconds = elapsed_time.as_secs_f64();
        let throughput_mbps = if seconds > 0.0 {
            (bytes_processed as f64) / (1024.0 * 1024.0) / seconds
        } else {
            0.0
        };
        
        let iops = if seconds > 0.0 {
            operations_count as f64 / seconds
        } else {
            0.0
        };
        
        Self {
            bytes_processed,
            elapsed_time,
            throughput_mbps,
            iops,
            operations_count,
        }
    }
    
    /// Combine multiple metrics
    pub fn combine(metrics: &[IOMetrics]) -> Self {
        let total_bytes: u64 = metrics.iter().map(|m| m.bytes_processed).sum();
        let total_ops: u64 = metrics.iter().map(|m| m.operations_count).sum();
        let max_time = metrics.iter()
            .map(|m| m.elapsed_time)
            .max()
            .unwrap_or(Duration::ZERO);
        
        Self::new(total_bytes, max_time, total_ops)
    }
}

/// Storage type detection for optimal block size selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageType {
    HDD,
    SSD,
    NVMe,
    Unknown,
}

impl StorageType {
    /// Get optimal block size for storage type
    pub fn optimal_block_size(&self) -> u64 {
        match self {
            StorageType::HDD => 1024 * 1024,      // 1MB for HDDs
            StorageType::SSD => 64 * 1024,        // 64KB for SATA SSDs
            StorageType::NVMe => 128 * 1024,      // 128KB for NVMe
            StorageType::Unknown => 64 * 1024,    // Conservative default
        }
    }
    
    /// Get optimal queue depth for storage type
    pub fn optimal_queue_depth(&self) -> usize {
        match self {
            StorageType::HDD => 1,      // HDDs work best with sequential access
            StorageType::SSD => 4,      // SATA SSDs benefit from some parallelism
            StorageType::NVMe => 8,     // NVMe can handle higher queue depths
            StorageType::Unknown => 2,  // Conservative default
        }
    }
}

/// Detect storage type based on path (simplified heuristic)
pub async fn detect_storage_type(_path: &Path) -> io::Result<StorageType> {
    // This is a simplified implementation
    // In a real implementation, you would query system APIs to determine
    // the actual storage device type
    
    task::spawn_blocking(move || {
        // For now, return a reasonable default
        // TODO: Implement actual storage detection using platform APIs
        Ok(StorageType::SSD)
    }).await
    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::disk::PlatformDiskIO;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_async_disk_io_creation() {
        let disk_io = PlatformDiskIO::new();
        let async_io = AsyncDiskIO::new(disk_io, 4096).unwrap();
        
        assert_eq!(async_io.buffer_pool().buffer_size(), 4096);
    }
    
    #[tokio::test]
    async fn test_pooled_buffer() {
        let disk_io = PlatformDiskIO::new();
        let async_io = AsyncDiskIO::new(disk_io, 1024).unwrap();
        
        let buffer = async_io.get_pooled_buffer().await.unwrap();
        assert_eq!(buffer.len(), 1024);
    }
    
    #[tokio::test]
    async fn test_temp_file_creation_async() {
        let temp_dir = tempdir().unwrap();
        let disk_io = PlatformDiskIO::new();
        let async_io = AsyncDiskIO::new(disk_io, 4096).unwrap();
        
        let temp_file = async_io.create_temp_file_async(temp_dir.path(), 1024).await.unwrap();
        assert!(temp_file.path().exists());
    }
    
    #[test]
    fn test_io_metrics() {
        let metrics = IOMetrics::new(1024 * 1024, Duration::from_secs(1), 100);
        assert_eq!(metrics.bytes_processed, 1024 * 1024);
        assert!((metrics.throughput_mbps - 1.0).abs() < 0.01);
        assert!((metrics.iops - 100.0).abs() < 0.01);
    }
    
    #[test]
    fn test_storage_type_block_sizes() {
        assert_eq!(StorageType::HDD.optimal_block_size(), 1024 * 1024);
        assert_eq!(StorageType::SSD.optimal_block_size(), 64 * 1024);
        assert_eq!(StorageType::NVMe.optimal_block_size(), 128 * 1024);
        assert_eq!(StorageType::Unknown.optimal_block_size(), 64 * 1024);
    }
    
    #[test]
    fn test_metrics_combine() {
        let metrics1 = IOMetrics::new(1024, Duration::from_millis(100), 10);
        let metrics2 = IOMetrics::new(2048, Duration::from_millis(200), 20);
        
        let combined = IOMetrics::combine(&[metrics1, metrics2]);
        assert_eq!(combined.bytes_processed, 3072);
        assert_eq!(combined.operations_count, 30);
        assert_eq!(combined.elapsed_time, Duration::from_millis(200));
    }
}