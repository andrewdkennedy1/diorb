//! Configuration management module
//! 
//! Handles loading, saving, and validation of benchmark configuration
//! and user preferences.

use std::path::PathBuf;
use std::time::Duration;
use std::fs;
use serde::{Deserialize, Serialize};
use crate::{DIOrbError, Result, APP_NAME, CONFIG_FILE};

pub mod persistence;

use crate::models::BenchmarkResult;

/// Benchmark configuration structure containing all test parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Target disk/directory path for testing
    pub disk_path: PathBuf,
    /// Benchmark mode (sequential, random, mixed)
    pub mode: BenchmarkMode,
    /// Total file size for testing (in bytes)
    pub file_size: u64,
    /// Block size for I/O operations (in bytes)
    pub block_size: u64,
    /// Test duration for time-based benchmarks
    pub duration: Duration,
    /// Number of concurrent threads/workers
    pub thread_count: usize,
    /// Whether to keep temporary files after testing
    pub keep_temp_files: bool,
}

/// Benchmark mode variants for different test types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenchmarkMode {
    /// Sequential write operations
    SequentialWrite,
    /// Sequential read operations  
    SequentialRead,
    /// Random read and write operations
    RandomReadWrite,
    /// Mixed read/write operations with configurable ratio
    Mixed { 
        /// Read operation ratio (0.0 to 1.0)
        read_ratio: f32 
    },
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            disk_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            mode: BenchmarkMode::SequentialWrite,
            file_size: 1024 * 1024 * 1024, // 1 GiB
            block_size: 64 * 1024, // 64 KiB
            duration: Duration::from_secs(30),
            thread_count: 1,
            keep_temp_files: false,
        }
    }
}

impl BenchmarkConfig {
    /// Create a new benchmark configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create configuration for sequential write benchmark
    pub fn sequential_write() -> Self {
        Self {
            mode: BenchmarkMode::SequentialWrite,
            file_size: 1024 * 1024 * 1024, // 1 GiB
            block_size: 64 * 1024, // 64 KiB
            ..Self::default()
        }
    }

    /// Create configuration for sequential read benchmark
    pub fn sequential_read() -> Self {
        Self {
            mode: BenchmarkMode::SequentialRead,
            file_size: 1024 * 1024 * 1024, // 1 GiB
            block_size: 64 * 1024, // 64 KiB
            ..Self::default()
        }
    }

    /// Create configuration for random read/write benchmark
    pub fn random_read_write() -> Self {
        Self {
            mode: BenchmarkMode::RandomReadWrite,
            block_size: 4 * 1024, // 4 KiB
            duration: Duration::from_secs(30),
            ..Self::default()
        }
    }

    /// Create configuration for mixed read/write benchmark
    pub fn mixed(read_ratio: f32) -> Self {
        Self {
            mode: BenchmarkMode::Mixed { read_ratio },
            block_size: 4 * 1024, // 4 KiB
            duration: Duration::from_secs(30),
            thread_count: 4,
            ..Self::default()
        }
    }

    /// Validate the configuration parameters
    pub fn validate(&self) -> Result<()> {
        // Validate disk path exists and is accessible
        if !self.disk_path.exists() {
            return Err(DIOrbError::ConfigError(
                format!("Disk path does not exist: {}", self.disk_path.display())
            ));
        }

        if !self.disk_path.is_dir() {
            return Err(DIOrbError::ConfigError(
                format!("Disk path is not a directory: {}", self.disk_path.display())
            ));
        }

        // Validate file size constraints
        if self.file_size == 0 {
            return Err(DIOrbError::ConfigError(
                "File size must be greater than 0".to_string()
            ));
        }

        // File size should be reasonable (not exceed 100 GiB)
        const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024 * 1024; // 100 GiB
        if self.file_size > MAX_FILE_SIZE {
            return Err(DIOrbError::ConfigError(
                format!("File size too large: {} bytes (max: {} bytes)", 
                    self.file_size, MAX_FILE_SIZE)
            ));
        }

        // Validate block size constraints
        if self.block_size == 0 {
            return Err(DIOrbError::ConfigError(
                "Block size must be greater than 0".to_string()
            ));
        }

        // Block size should be power of 2 and within reasonable range
        if !self.block_size.is_power_of_two() {
            return Err(DIOrbError::ConfigError(
                "Block size must be a power of 2".to_string()
            ));
        }

        const MIN_BLOCK_SIZE: u64 = 512; // 512 bytes
        const MAX_BLOCK_SIZE: u64 = 1024 * 1024; // 1 MiB
        if self.block_size < MIN_BLOCK_SIZE || self.block_size > MAX_BLOCK_SIZE {
            return Err(DIOrbError::ConfigError(
                format!("Block size must be between {} and {} bytes", 
                    MIN_BLOCK_SIZE, MAX_BLOCK_SIZE)
            ));
        }

        // For sequential operations, file size should be larger than block size
        match self.mode {
            BenchmarkMode::SequentialWrite | BenchmarkMode::SequentialRead => {
                if self.file_size < self.block_size {
                    return Err(DIOrbError::ConfigError(
                        "File size must be larger than block size for sequential operations".to_string()
                    ));
                }
            }
            _ => {}
        }

        // Validate duration constraints
        if self.duration.is_zero() {
            return Err(DIOrbError::ConfigError(
                "Duration must be greater than 0".to_string()
            ));
        }

        const MAX_DURATION: Duration = Duration::from_secs(3600); // 1 hour
        if self.duration > MAX_DURATION {
            return Err(DIOrbError::ConfigError(
                format!("Duration too long: {}s (max: {}s)", 
                    self.duration.as_secs(), MAX_DURATION.as_secs())
            ));
        }

        // Validate thread count constraints
        if self.thread_count == 0 {
            return Err(DIOrbError::ConfigError(
                "Thread count must be greater than 0".to_string()
            ));
        }

        const MAX_THREADS: usize = 64;
        if self.thread_count > MAX_THREADS {
            return Err(DIOrbError::ConfigError(
                format!("Too many threads: {} (max: {})", 
                    self.thread_count, MAX_THREADS)
            ));
        }

        // Validate mode-specific constraints
        match &self.mode {
            BenchmarkMode::Mixed { read_ratio } => {
                if *read_ratio < 0.0 || *read_ratio > 1.0 {
                    return Err(DIOrbError::ConfigError(
                        "Read ratio must be between 0.0 and 1.0".to_string()
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Set the disk path for testing
    pub fn with_disk_path(mut self, path: PathBuf) -> Self {
        self.disk_path = path;
        self
    }

    /// Set the benchmark mode
    pub fn with_mode(mut self, mode: BenchmarkMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the file size for testing
    pub fn with_file_size(mut self, size: u64) -> Self {
        self.file_size = size;
        self
    }

    /// Set the block size for I/O operations
    pub fn with_block_size(mut self, size: u64) -> Self {
        self.block_size = size;
        self
    }

    /// Set the test duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set the number of threads
    pub fn with_thread_count(mut self, count: usize) -> Self {
        self.thread_count = count;
        self
    }

    /// Set whether to keep temporary files
    pub fn with_keep_temp_files(mut self, keep: bool) -> Self {
        self.keep_temp_files = keep;
        self
    }

    /// Load configuration from the standard config file location
    /// Returns default configuration if file doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::config_file_path()?;
        
        if !config_path.exists() {
            // Return default configuration if file doesn't exist
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| DIOrbError::ConfigError(
                format!("Failed to read config file {}: {}", config_path.display(), e)
            ))?;

        let config: Self = toml::from_str(&content)
            .map_err(|e| DIOrbError::ConfigError(
                format!("Failed to parse config file {}: {}", config_path.display(), e)
            ))?;

        // Validate the loaded configuration
        config.validate()?;
        
        Ok(config)
    }

    /// Save configuration to the standard config file location
    pub fn save(&self) -> Result<()> {
        // Validate before saving
        self.validate()?;

        let config_path = Self::config_file_path()?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| DIOrbError::ConfigError(
                    format!("Failed to create config directory {}: {}", parent.display(), e)
                ))?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| DIOrbError::ConfigError(
                format!("Failed to serialize configuration: {}", e)
            ))?;

        fs::write(&config_path, content)
            .map_err(|e| DIOrbError::ConfigError(
                format!("Failed to write config file {}: {}", config_path.display(), e)
            ))?;

        Ok(())
    }

    /// Get the standard configuration file path
    /// Uses $CONFIG_HOME/diorb.toml or falls back to $HOME/.config/diorb.toml
    pub fn config_file_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| DIOrbError::ConfigError(
                "Unable to determine config directory".to_string()
            ))?;

        Ok(config_dir.join(APP_NAME).join(CONFIG_FILE))
    }
}

impl BenchmarkMode {
    /// Check if this mode uses file size (sequential operations)
    pub fn uses_file_size(&self) -> bool {
        matches!(self, BenchmarkMode::SequentialWrite | BenchmarkMode::SequentialRead)
    }

    /// Check if this mode uses duration (time-based operations)
    pub fn uses_duration(&self) -> bool {
        matches!(self, BenchmarkMode::RandomReadWrite | BenchmarkMode::Mixed { .. })
    }

    /// Get the default block size for this mode
    pub fn default_block_size(&self) -> u64 {
        match self {
            BenchmarkMode::SequentialWrite | BenchmarkMode::SequentialRead => 64 * 1024, // 64 KiB
            BenchmarkMode::RandomReadWrite | BenchmarkMode::Mixed { .. } => 4 * 1024, // 4 KiB
        }
    }

    /// Get the default thread count for this mode
    pub fn default_thread_count(&self) -> usize {
        match self {
            BenchmarkMode::SequentialWrite | BenchmarkMode::SequentialRead => 1,
            BenchmarkMode::RandomReadWrite => 1,
            BenchmarkMode::Mixed { .. } => 4,
        }
    }

    /// Get a human-readable description of the mode
    pub fn description(&self) -> &'static str {
        match self {
            BenchmarkMode::SequentialWrite => "Sequential Write",
            BenchmarkMode::SequentialRead => "Sequential Read",
            BenchmarkMode::RandomReadWrite => "Random Read/Write",
            BenchmarkMode::Mixed { .. } => "Mixed Read/Write",
        }
    }
}

/// Configuration manager for handling config and results persistence
pub struct ConfigManager {
    config_path: PathBuf,
    results_manager: persistence::ResultsStorage,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Result<Self> {
        let config_path = BenchmarkConfig::config_file_path()?;
        let results_manager = persistence::ResultsStorage::new()?;
        
        Ok(Self {
            config_path,
            results_manager,
        })
    }

    /// Load configuration from file or return default
    pub fn load_config(&self) -> Result<BenchmarkConfig> {
        BenchmarkConfig::load()
    }

    /// Save configuration to file
    pub fn save_config(&self, config: &BenchmarkConfig) -> Result<()> {
        config.save()
    }

    /// Save a benchmark result
    pub fn save_result(&self, result: BenchmarkResult) -> Result<()> {
        self.results_manager.append_result(result)
    }

    /// Load all saved results
    pub fn load_results(&self) -> Result<Vec<BenchmarkResult>> {
        self.results_manager.load_results()
    }

    /// Get the most recent results (up to limit)
    pub fn get_recent_results(&self, limit: usize) -> Result<Vec<BenchmarkResult>> {
        let mut results = self.load_results()?;
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Most recent first
        results.truncate(limit);
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_serialization() {
        let config = BenchmarkConfig::mixed(0.7);
        let json = serde_json::to_string(&config).expect("Failed to serialize");
        let deserialized: BenchmarkConfig = serde_json::from_str(&json).expect("Failed to deserialize");
        
        if let (BenchmarkMode::Mixed { read_ratio: original }, BenchmarkMode::Mixed { read_ratio: deserialized_ratio }) = (&config.mode, &deserialized.mode) {
            assert_eq!(original, deserialized_ratio);
        } else {
            panic!("Mode serialization failed");
        }
    }

    #[test]
    fn test_toml_serialization() {
        let config = BenchmarkConfig::mixed(0.7);
        let toml_str = toml::to_string(&config).expect("Failed to serialize to TOML");
        let deserialized: BenchmarkConfig = toml::from_str(&toml_str).expect("Failed to deserialize from TOML");
        
        if let (BenchmarkMode::Mixed { read_ratio: original }, BenchmarkMode::Mixed { read_ratio: deserialized_ratio }) = (&config.mode, &deserialized.mode) {
            assert_eq!(original, deserialized_ratio);
        } else {
            panic!("TOML mode serialization failed");
        }
        
        assert_eq!(config.file_size, deserialized.file_size);
        assert_eq!(config.block_size, deserialized.block_size);
        assert_eq!(config.thread_count, deserialized.thread_count);
        assert_eq!(config.keep_temp_files, deserialized.keep_temp_files);
    }

    #[test]
    fn test_config_file_path() {
        let path = BenchmarkConfig::config_file_path();
        assert!(path.is_ok());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("diorb"));
        assert!(path.to_string_lossy().contains("diorb.toml"));
    }
}
