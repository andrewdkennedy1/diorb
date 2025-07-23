use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use crate::{DIOrbError, Result};

/// Buffer pool for reusing allocated buffers to reduce memory allocation overhead
pub struct BufferPool {
    buffers: Arc<Mutex<VecDeque<Vec<u8>>>>,
    buffer_size: usize,
    max_buffers: usize,
}

impl BufferPool {
    /// Create a new buffer pool with specified buffer size and maximum count
    pub fn new(buffer_size: usize, max_buffers: usize) -> Result<Self> {
        if buffer_size == 0 {
            return Err(DIOrbError::ConfigError("Buffer size must be greater than 0".to_string()));
        }
        if max_buffers == 0 {
            return Err(DIOrbError::ConfigError("Max buffers must be greater than 0".to_string()));
        }
        
        Ok(Self {
            buffers: Arc::new(Mutex::new(VecDeque::new())),
            buffer_size,
            max_buffers,
        })
    }
    
    /// Get a buffer from the pool, creating a new one if none available
    pub async fn get_buffer(&self) -> Result<Vec<u8>> {
        // For now, this is synchronous but we keep the async signature for future improvements
        let mut buffers = self.buffers.lock()
            .map_err(|_| DIOrbError::BenchmarkError("Buffer pool lock poisoned".to_string()))?;
        
        let buffer = buffers.pop_front().unwrap_or_else(|| {
            let mut buffer = Vec::with_capacity(self.buffer_size);
            buffer.resize(self.buffer_size, 0);
            buffer
        });
        
        Ok(buffer)
    }
    
    /// Return a buffer to the pool for reuse
    pub fn return_buffer(&self, mut buffer: Vec<u8>) -> Result<()> {
        if buffer.len() == self.buffer_size {
            let mut buffers = self.buffers.lock()
                .map_err(|_| DIOrbError::BenchmarkError("Buffer pool lock poisoned".to_string()))?;
            
            if buffers.len() < self.max_buffers {
                // Clear the buffer for security
                buffer.fill(0);
                buffers.push_back(buffer);
            }
        }
        Ok(())
    }
    
    /// Get the configured buffer size
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }
    
    /// Get current number of pooled buffers
    pub fn pool_size(&self) -> Result<usize> {
        let buffers = self.buffers.lock()
            .map_err(|_| DIOrbError::BenchmarkError("Buffer pool lock poisoned".to_string()))?;
        Ok(buffers.len())
    }
}

impl Clone for BufferPool {
    fn clone(&self) -> Self {
        Self {
            buffers: Arc::clone(&self.buffers),
            buffer_size: self.buffer_size,
            max_buffers: self.max_buffers,
        }
    }
}

/// RAII wrapper for buffer pool management
pub struct PooledBuffer {
    buffer: Option<Vec<u8>>,
    pool: BufferPool,
}

impl PooledBuffer {
    pub async fn new(pool: BufferPool) -> Result<Self> {
        let buffer = pool.get_buffer().await?;
        Ok(Self {
            buffer: Some(buffer),
            pool,
        })
    }
    
    /// Get mutable access to the buffer
    pub fn as_mut(&mut self) -> &mut [u8] {
        self.buffer.as_mut().unwrap()
    }
    
    /// Get immutable access to the buffer
    pub fn as_ref(&self) -> &[u8] {
        self.buffer.as_ref().unwrap()
    }
    
    /// Get the buffer size
    pub fn len(&self) -> usize {
        self.buffer.as_ref().unwrap().len()
    }
    
    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            let _ = self.pool.return_buffer(buffer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_buffer_pool_basic() {
        let pool = BufferPool::new(1024, 5).unwrap();
        
        let buffer1 = pool.get_buffer().await.unwrap();
        assert_eq!(buffer1.len(), 1024);
        assert_eq!(pool.pool_size().unwrap(), 0);
        
        pool.return_buffer(buffer1).unwrap();
        assert_eq!(pool.pool_size().unwrap(), 1);
        
        let buffer2 = pool.get_buffer().await.unwrap();
        assert_eq!(buffer2.len(), 1024);
        assert_eq!(pool.pool_size().unwrap(), 0);
    }
    
    #[tokio::test]
    async fn test_buffer_pool_max_limit() {
        let pool = BufferPool::new(512, 2).unwrap();
        
        let buf1 = pool.get_buffer().await.unwrap();
        let buf2 = pool.get_buffer().await.unwrap();
        let buf3 = pool.get_buffer().await.unwrap();
        
        pool.return_buffer(buf1).unwrap();
        pool.return_buffer(buf2).unwrap();
        pool.return_buffer(buf3).unwrap();
        
        // Should only keep 2 buffers due to max_buffers limit
        assert_eq!(pool.pool_size().unwrap(), 2);
    }
    
    #[tokio::test]
    async fn test_pooled_buffer_raii() {
        let pool = BufferPool::new(256, 3).unwrap();
        
        {
            let mut pooled = PooledBuffer::new(pool.clone()).await.unwrap();
            assert_eq!(pooled.len(), 256);
            pooled.as_mut()[0] = 42;
            assert_eq!(pooled.as_ref()[0], 42);
        }
        
        // Buffer should be returned to pool automatically
        assert_eq!(pool.pool_size().unwrap(), 1);
        
        // Next buffer should be cleared
        let pooled2 = PooledBuffer::new(pool.clone()).await.unwrap();
        assert_eq!(pooled2.as_ref()[0], 0);
    }
}