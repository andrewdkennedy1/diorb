//! Results persistence module
//! 
//! Handles saving, loading, and rotation of benchmark results.

use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::{DIOrbError, Result, APP_NAME, RESULTS_FILE, MAX_RESULTS_HISTORY};
use crate::models::result::BenchmarkResult;

/// Results storage manager
#[derive(Debug)]
pub struct ResultsStorage {
    results_path: PathBuf,
}

/// Results file structure for JSON persistence
#[derive(Debug, Serialize, Deserialize)]
struct ResultsFile {
    version: u32,
    results: Vec<BenchmarkResult>,
}

impl Default for ResultsFile {
    fn default() -> Self {
        Self {
            version: 1,
            results: Vec::new(),
        }
    }
}

impl ResultsStorage {
    /// Create a new results storage manager
    pub fn new() -> Result<Self> {
        let results_path = Self::results_file_path()?;
        Ok(Self { results_path })
    }

    /// Get the standard results file path
    /// Uses $DATA_HOME/diorb/results.json or falls back to $HOME/.local/share/diorb/results.json
    pub fn results_file_path() -> Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| DIOrbError::ConfigError(
                "Unable to determine data directory".to_string()
            ))?;

        Ok(data_dir.join(APP_NAME).join(RESULTS_FILE))
    }

    /// Load all results from the results file
    pub fn load_results(&self) -> Result<Vec<BenchmarkResult>> {
        if !self.results_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.results_path)
            .map_err(|e| DIOrbError::ConfigError(
                format!("Failed to read results file {}: {}", self.results_path.display(), e)
            ))?;

        let results_file: ResultsFile = serde_json::from_str(&content)
            .map_err(|e| DIOrbError::ConfigError(
                format!("Failed to parse results file {}: {}", self.results_path.display(), e)
            ))?;

        Ok(results_file.results)
    }

    /// Append a new result to the results file
    /// Automatically rotates old results if the file exceeds MAX_RESULTS_HISTORY entries
    pub fn append_result(&self, result: BenchmarkResult) -> Result<()> {
        let mut results = self.load_results()?;
        
        // Add the new result
        results.push(result);
        
        // Rotate if we exceed the maximum history
        if results.len() > MAX_RESULTS_HISTORY {
            // Keep only the most recent MAX_RESULTS_HISTORY results
            let skip_count = results.len() - MAX_RESULTS_HISTORY;
            results = results.into_iter()
                .skip(skip_count)
                .collect();
        }

        self.save_results(results)
    }

    /// Save all results to the results file
    fn save_results(&self, results: Vec<BenchmarkResult>) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = self.results_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| DIOrbError::ConfigError(
                    format!("Failed to create results directory {}: {}", parent.display(), e)
                ))?;
        }

        let results_file = ResultsFile {
            version: 1,
            results,
        };

        let content = serde_json::to_string_pretty(&results_file)
            .map_err(|e| DIOrbError::ConfigError(
                format!("Failed to serialize results: {}", e)
            ))?;

        fs::write(&self.results_path, content)
            .map_err(|e| DIOrbError::ConfigError(
                format!("Failed to write results file {}: {}", self.results_path.display(), e)
            ))?;

        Ok(())
    }

    /// Get the number of stored results
    pub fn count_results(&self) -> Result<usize> {
        let results = self.load_results()?;
        Ok(results.len())
    }

    /// Clear all stored results
    pub fn clear_results(&self) -> Result<()> {
        if self.results_path.exists() {
            fs::remove_file(&self.results_path)
                .map_err(|e| DIOrbError::ConfigError(
                    format!("Failed to remove results file {}: {}", self.results_path.display(), e)
                ))?;
        }
        Ok(())
    }

    /// Get the most recent N results
    pub fn get_recent_results(&self, count: usize) -> Result<Vec<BenchmarkResult>> {
        let results = self.load_results()?;
        
        if results.len() <= count {
            Ok(results)
        } else {
            let skip_count = results.len() - count;
            Ok(results.into_iter()
                .skip(skip_count)
                .collect())
        }
    }

    /// Get results file path for external access
    pub fn get_results_path(&self) -> &PathBuf {
        &self.results_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BenchmarkConfig;
    use crate::models::result::{BenchmarkResult, PerformanceMetrics, LatencyStats};
    use chrono::Utc;
    use std::time::Duration;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_result() -> BenchmarkResult {
        BenchmarkResult {
            timestamp: Utc::now(),
            config: BenchmarkConfig::default(),
            metrics: PerformanceMetrics {
                bytes_processed: 1024 * 1024 * 1024, // 1 GiB
                elapsed_time: Duration::from_secs(10),
                throughput_mbps: 100.0,
                iops: 1600.0,
                latency: LatencyStats {
                    min: Duration::from_millis(1),
                    avg: Duration::from_millis(5),
                    max: Duration::from_millis(20),
                    percentiles: {
                        let mut map = HashMap::new();
                        map.insert(50, Duration::from_millis(4));
                        map.insert(95, Duration::from_millis(15));
                        map.insert(99, Duration::from_millis(18));
                        map
                    },
                },
            },
            system_info: Default::default(),
        }
    }

    #[test]
    fn test_results_storage_new() {
        let storage = ResultsStorage::new();
        assert!(storage.is_ok());
    }

    #[test]
    fn test_load_empty_results() {
        let temp_dir = TempDir::new().unwrap();
        let results_path = temp_dir.path().join("results.json");
        
        let storage = ResultsStorage { results_path };
        let results = storage.load_results().unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_append_and_load_result() {
        let temp_dir = TempDir::new().unwrap();
        let results_path = temp_dir.path().join("results.json");
        
        let storage = ResultsStorage { results_path };
        let test_result = create_test_result();
        
        // Append result
        storage.append_result(test_result.clone()).unwrap();
        
        // Load and verify
        let results = storage.load_results().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metrics.bytes_processed, test_result.metrics.bytes_processed);
    }

    #[test]
    fn test_results_rotation() {
        let temp_dir = TempDir::new().unwrap();
        let results_path = temp_dir.path().join("results.json");
        
        let storage = ResultsStorage { results_path };
        
        // Add more than MAX_RESULTS_HISTORY results
        for i in 0..MAX_RESULTS_HISTORY + 10 {
            let mut result = create_test_result();
            result.metrics.bytes_processed = i as u64; // Make each result unique
            storage.append_result(result).unwrap();
        }
        
        // Verify only MAX_RESULTS_HISTORY results are kept
        let results = storage.load_results().unwrap();
        assert_eq!(results.len(), MAX_RESULTS_HISTORY);
        
        // Verify the oldest results were removed (first 10 should be gone)
        assert_eq!(results[0].metrics.bytes_processed, 10);
        assert_eq!(results[results.len() - 1].metrics.bytes_processed, (MAX_RESULTS_HISTORY + 10 - 1) as u64);
    }

    #[test]
    fn test_count_results() {
        let temp_dir = TempDir::new().unwrap();
        let results_path = temp_dir.path().join("results.json");
        
        let storage = ResultsStorage { results_path };
        
        // Initially empty
        assert_eq!(storage.count_results().unwrap(), 0);
        
        // Add some results
        for _ in 0..5 {
            storage.append_result(create_test_result()).unwrap();
        }
        
        assert_eq!(storage.count_results().unwrap(), 5);
    }

    #[test]
    fn test_clear_results() {
        let temp_dir = TempDir::new().unwrap();
        let results_path = temp_dir.path().join("results.json");
        
        let storage = ResultsStorage { results_path };
        
        // Add some results
        for _ in 0..3 {
            storage.append_result(create_test_result()).unwrap();
        }
        
        assert_eq!(storage.count_results().unwrap(), 3);
        
        // Clear results
        storage.clear_results().unwrap();
        assert_eq!(storage.count_results().unwrap(), 0);
    }

    #[test]
    fn test_get_recent_results() {
        let temp_dir = TempDir::new().unwrap();
        let results_path = temp_dir.path().join("results.json");
        
        let storage = ResultsStorage { results_path };
        
        // Add 10 results with unique identifiers
        for i in 0..10 {
            let mut result = create_test_result();
            result.metrics.bytes_processed = i as u64;
            storage.append_result(result).unwrap();
        }
        
        // Get recent 5 results
        let recent = storage.get_recent_results(5).unwrap();
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[0].metrics.bytes_processed, 5); // Should be results 5-9
        assert_eq!(recent[4].metrics.bytes_processed, 9);
        
        // Get more results than available
        let all_recent = storage.get_recent_results(20).unwrap();
        assert_eq!(all_recent.len(), 10);
    }

    #[test]
    fn test_results_file_format() {
        let temp_dir = TempDir::new().unwrap();
        let results_path = temp_dir.path().join("results.json");
        
        let storage = ResultsStorage { results_path: results_path.clone() };
        let test_result = create_test_result();
        
        storage.append_result(test_result).unwrap();
        
        // Verify the file format
        let content = fs::read_to_string(&results_path).unwrap();
        let results_file: ResultsFile = serde_json::from_str(&content).unwrap();
        
        assert_eq!(results_file.version, 1);
        assert_eq!(results_file.results.len(), 1);
    }
}