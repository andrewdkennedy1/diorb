use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Cross-platform disk I/O operations trait
pub trait DiskIO {
    /// Open a file for direct write operations (bypassing OS cache)
    fn open_direct_write(&self, path: &Path) -> io::Result<Box<dyn DirectFile>>;
    
    /// Open a file for direct read operations (bypassing OS cache)
    fn open_direct_read(&self, path: &Path) -> io::Result<Box<dyn DirectFile>>;
    
    /// Create a temporary file for benchmarking
    fn create_temp_file(&self, target_dir: &Path, size_hint: u64) -> io::Result<TempFile>;
    
    /// Get optimal block size for the given path
    fn get_optimal_block_size(&self, path: &Path) -> io::Result<u64>;
}

/// Direct file operations trait for unbuffered I/O
pub trait DirectFile: Send + Sync {
    /// Write data directly to disk
    fn write_direct(&mut self, buf: &[u8]) -> io::Result<usize>;
    
    /// Read data directly from disk
    fn read_direct(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    
    /// Seek to position
    fn seek_direct(&mut self, pos: SeekFrom) -> io::Result<u64>;
    
    /// Force synchronization to disk
    fn sync_all(&mut self) -> io::Result<()>;
    
    /// Get file size
    fn file_size(&self) -> io::Result<u64>;
}

/// Temporary file wrapper with automatic cleanup
pub struct TempFile {
    pub path: PathBuf,
    pub file: Box<dyn DirectFile>,
    cleanup_on_drop: bool,
}

impl TempFile {
    pub fn new(path: PathBuf, file: Box<dyn DirectFile>, cleanup: bool) -> Self {
        Self {
            path,
            file,
            cleanup_on_drop: cleanup,
        }
    }
    
    /// Disable automatic cleanup (for debugging)
    pub fn keep_on_drop(&mut self) {
        self.cleanup_on_drop = false;
    }
    
    /// Get the file path
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// Platform-specific disk I/O implementation
#[derive(Clone)]
pub struct PlatformDiskIO;

impl PlatformDiskIO {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlatformDiskIO {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;
    
    const FILE_FLAG_WRITE_THROUGH: u32 = 0x80000000;
    const FILE_FLAG_NO_BUFFERING: u32 = 0x20000000;
    
    pub struct WindowsDirectFile {
        file: File,
    }
    
    impl WindowsDirectFile {
        pub fn new(file: File) -> Self {
            Self { file }
        }
    }
    
    impl DirectFile for WindowsDirectFile {
        fn write_direct(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.file.write(buf)
        }
        
        fn read_direct(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.file.read(buf)
        }
        
        fn seek_direct(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.file.seek(pos)
        }
        
        fn sync_all(&mut self) -> io::Result<()> {
            self.file.sync_all()
        }
        
        fn file_size(&self) -> io::Result<u64> {
            Ok(self.file.metadata()?.len())
        }
    }
    
    impl DiskIO for PlatformDiskIO {
        fn open_direct_write(&self, path: &Path) -> io::Result<Box<dyn DirectFile>> {
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .custom_flags(FILE_FLAG_WRITE_THROUGH | FILE_FLAG_NO_BUFFERING)
                .open(path)?;
            
            Ok(Box::new(WindowsDirectFile::new(file)))
        }
        
        fn open_direct_read(&self, path: &Path) -> io::Result<Box<dyn DirectFile>> {
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(FILE_FLAG_NO_BUFFERING)
                .open(path)?;
            
            Ok(Box::new(WindowsDirectFile::new(file)))
        }
        
        fn create_temp_file(&self, target_dir: &Path, _size_hint: u64) -> io::Result<TempFile> {
            use std::process;
            
            let temp_name = format!("DIORB_TMP_{}.dat", process::id());
            let temp_path = target_dir.join(temp_name);
            
            let file = self.open_direct_write(&temp_path)?;
            Ok(TempFile::new(temp_path, file, true))
        }
        
        fn get_optimal_block_size(&self, _path: &Path) -> io::Result<u64> {
            // Windows typically works well with 64KB blocks for sequential I/O
            Ok(65536)
        }
    }
}

#[cfg(unix)]
mod unix_impl {
    use super::*;
    use std::os::unix::fs::OpenOptionsExt;
    
    pub struct UnixDirectFile {
        file: File,
        use_fsync: bool,
    }
    
    impl UnixDirectFile {
        pub fn new(file: File, use_fsync: bool) -> Self {
            Self { file, use_fsync }
        }
    }
    
    impl DirectFile for UnixDirectFile {
        fn write_direct(&mut self, buf: &[u8]) -> io::Result<usize> {
            let result = self.file.write(buf)?;
            if self.use_fsync {
                self.file.sync_all()?;
            }
            Ok(result)
        }
        
        fn read_direct(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.file.read(buf)
        }
        
        fn seek_direct(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.file.seek(pos)
        }
        
        fn sync_all(&mut self) -> io::Result<()> {
            self.file.sync_all()
        }
        
        fn file_size(&self) -> io::Result<u64> {
            Ok(self.file.metadata()?.len())
        }
    }
    
    impl DiskIO for PlatformDiskIO {
        fn open_direct_write(&self, path: &Path) -> io::Result<Box<dyn DirectFile>> {
            // Try O_DIRECT first, fall back to regular file with fsync
            match OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .custom_flags(libc::O_DIRECT)
                .open(path)
            {
                Ok(file) => Ok(Box::new(UnixDirectFile::new(file, false))),
                Err(_) => {
                    // Fallback to regular file with fsync
                    let file = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(path)?;
                    Ok(Box::new(UnixDirectFile::new(file, true)))
                }
            }
        }
        
        fn open_direct_read(&self, path: &Path) -> io::Result<Box<dyn DirectFile>> {
            // Try O_DIRECT first, fall back to regular file
            match OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECT)
                .open(path)
            {
                Ok(file) => Ok(Box::new(UnixDirectFile::new(file, false))),
                Err(_) => {
                    // Fallback to regular file
                    let file = OpenOptions::new()
                        .read(true)
                        .open(path)?;
                    Ok(Box::new(UnixDirectFile::new(file, false)))
                }
            }
        }
        
        fn create_temp_file(&self, target_dir: &Path, _size_hint: u64) -> io::Result<TempFile> {
            use std::process;
            
            let temp_name = format!("DIORB_TMP_{}.dat", process::id());
            let temp_path = target_dir.join(temp_name);
            
            let file = self.open_direct_write(&temp_path)?;
            Ok(TempFile::new(temp_path, file, true))
        }
        
        fn get_optimal_block_size(&self, _path: &Path) -> io::Result<u64> {
            // Unix systems typically work well with 64KB blocks for sequential I/O
            Ok(65536)
        }
    }
}

// Re-export platform-specific implementations
#[cfg(windows)]
pub use windows_impl::*;

#[cfg(unix)]
pub use unix_impl::*;

/// Create a new platform-specific disk I/O instance
pub fn create_disk_io() -> impl DiskIO {
    PlatformDiskIO::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_temp_file_creation() {
        let temp_dir = tempdir().unwrap();
        let disk_io = PlatformDiskIO::new();
        
        let temp_file = disk_io.create_temp_file(temp_dir.path(), 1024).unwrap();
        assert!(temp_file.path().exists());
        
        // File should be cleaned up when dropped
        let path = temp_file.path().to_owned();
        drop(temp_file);
        assert!(!path.exists());
    }
    
    #[test]
    fn test_temp_file_keep_on_drop() {
        let temp_dir = tempdir().unwrap();
        let disk_io = PlatformDiskIO::new();
        
        let mut temp_file = disk_io.create_temp_file(temp_dir.path(), 1024).unwrap();
        temp_file.keep_on_drop();
        
        let path = temp_file.path().to_owned();
        drop(temp_file);
        assert!(path.exists());
        
        // Clean up manually
        std::fs::remove_file(path).unwrap();
    }
    
    #[test]
    fn test_optimal_block_size() {
        let temp_dir = tempdir().unwrap();
        let disk_io = PlatformDiskIO::new();
        
        let block_size = disk_io.get_optimal_block_size(temp_dir.path()).unwrap();
        assert_eq!(block_size, 65536); // 64KB
    }
}