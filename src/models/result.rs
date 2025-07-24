//! Benchmark result data models
//!
//! Contains structures for storing and serializing benchmark results,
//! performance metrics, and latency statistics.

use crate::config::BenchmarkConfig;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Complete benchmark result containing configuration, metrics, and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Timestamp when the benchmark was executed
    pub timestamp: DateTime<Utc>,
    /// Configuration used for this benchmark
    pub config: BenchmarkConfig,
    /// Performance metrics collected during the benchmark
    pub metrics: PerformanceMetrics,
    /// System information at time of benchmark
    pub system_info: SystemInfo,
}

/// Performance metrics collected during benchmark execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total bytes processed during the benchmark
    pub bytes_processed: u64,
    /// Total elapsed time for the benchmark
    #[serde(with = "duration_serde")]
    pub elapsed_time: Duration,
    /// Throughput in megabytes per second
    pub throughput_mbps: f64,
    /// Input/output operations per second
    pub iops: f64,
    /// Latency statistics for I/O operations
    pub latency: LatencyStats,
}

/// Latency statistics with min/avg/max and percentiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    /// Minimum latency observed
    #[serde(with = "duration_serde")]
    pub min: Duration,
    /// Average latency across all operations
    #[serde(with = "duration_serde")]
    pub avg: Duration,
    /// Maximum latency observed
    #[serde(with = "duration_serde")]
    pub max: Duration,
    /// Latency percentiles (50th, 95th, 99th, etc.)
    #[serde(with = "percentiles_serde")]
    pub percentiles: HashMap<u8, Duration>,
}

/// System information captured at benchmark time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system name and version
    pub os: String,
    /// CPU information
    pub cpu: String,
    /// Total system memory in bytes
    pub memory_total: u64,
    /// Available system memory in bytes at benchmark time
    pub memory_available: u64,
    /// Storage device information
    pub storage_info: StorageInfo,
}

/// Storage device information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageInfo {
    /// Device name or identifier
    pub device: String,
    /// File system type
    pub filesystem: String,
    /// Total storage capacity in bytes
    pub total_space: u64,
    /// Available storage space in bytes
    pub available_space: u64,
}

impl BenchmarkResult {
    /// Create a new benchmark result with detected system info
    pub fn new(config: BenchmarkConfig, metrics: PerformanceMetrics) -> Self {
        Self {
            timestamp: Utc::now(),
            config,
            metrics,
            system_info: SystemInfo::detect(),
        }
    }

    /// Create a new benchmark result with custom system info
    pub fn with_system_info(
        config: BenchmarkConfig,
        metrics: PerformanceMetrics,
        system_info: SystemInfo,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            config,
            metrics,
            system_info,
        }
    }

    /// Get a human-readable summary of the benchmark result
    pub fn summary(&self) -> String {
        format!(
            "{} - {} - {:.2} MB/s - {:.0} IOPS - {:.2}ms avg latency",
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            self.config.mode.description(),
            self.metrics.throughput_mbps,
            self.metrics.iops,
            self.metrics.latency.avg.as_secs_f64() * 1000.0
        )
    }

    /// Check if this result meets accuracy requirements based on storage type
    pub fn meets_accuracy_requirements(&self, other_results: &[BenchmarkResult]) -> bool {
        if other_results.len() < 2 {
            return true; // Can't validate accuracy with less than 3 runs
        }

        let throughputs: Vec<f64> = other_results
            .iter()
            .chain(std::iter::once(self))
            .map(|r| r.metrics.throughput_mbps)
            .collect();

        let avg_throughput = throughputs.iter().sum::<f64>() / throughputs.len() as f64;
        let max_deviation = throughputs
            .iter()
            .map(|&t| ((t - avg_throughput) / avg_throughput).abs())
            .fold(0.0, f64::max);

        // Determine accuracy threshold based on storage type (inferred from performance)
        let accuracy_threshold = if avg_throughput > 1000.0 {
            0.03 // NVMe: ±3%
        } else if avg_throughput > 100.0 {
            0.05 // SATA SSD: ±5%
        } else {
            0.08 // HDD: ±8%
        };

        max_deviation <= accuracy_threshold
    }
}

impl PerformanceMetrics {
    /// Create new performance metrics
    pub fn new(bytes_processed: u64, elapsed_time: Duration, latency: LatencyStats) -> Self {
        let elapsed_secs = elapsed_time.as_secs_f64();
        let throughput_mbps = if elapsed_secs > 0.0 {
            (bytes_processed as f64) / (1024.0 * 1024.0) / elapsed_secs
        } else {
            0.0
        };

        let iops = if elapsed_secs > 0.0 && !latency.avg.is_zero() {
            1.0 / latency.avg.as_secs_f64()
        } else {
            0.0
        };

        Self {
            bytes_processed,
            elapsed_time,
            throughput_mbps,
            iops,
            latency,
        }
    }

    /// Get efficiency ratio (throughput per thread)
    pub fn efficiency_ratio(&self, thread_count: usize) -> f64 {
        if thread_count > 0 {
            self.throughput_mbps / thread_count as f64
        } else {
            0.0
        }
    }

    /// Validate that the stored throughput is consistent with bytes and time
    pub fn validate_throughput(&self) -> bool {
        use crate::util::units::calculate_throughput_mbps;

        if self.elapsed_time.is_zero() {
            return self.throughput_mbps == 0.0;
        }

        let expected = calculate_throughput_mbps(self.bytes_processed, self.elapsed_time);
        if expected == 0.0 {
            return self.throughput_mbps == 0.0;
        }

        ((self.throughput_mbps - expected) / expected).abs() < 0.01
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            bytes_processed: 0,
            elapsed_time: Duration::default(),
            throughput_mbps: 0.0,
            iops: 0.0,
            latency: LatencyStats {
                min: Duration::default(),
                avg: Duration::default(),
                max: Duration::default(),
                percentiles: HashMap::new(),
            },
        }
    }
}

impl LatencyStats {
    /// Create new latency statistics
    pub fn new(min: Duration, avg: Duration, max: Duration) -> Self {
        let mut percentiles = HashMap::new();
        percentiles.insert(50, avg); // Use avg as 50th percentile approximation
        percentiles.insert(95, max); // Use max as 95th percentile approximation
        percentiles.insert(99, max); // Use max as 99th percentile approximation

        Self {
            min,
            avg,
            max,
            percentiles,
        }
    }

    /// Create latency statistics with custom percentiles
    pub fn with_percentiles(
        min: Duration,
        avg: Duration,
        max: Duration,
        percentiles: HashMap<u8, Duration>,
    ) -> Self {
        Self {
            min,
            avg,
            max,
            percentiles,
        }
    }

    /// Check if latency meets accuracy requirements for storage type
    pub fn meets_latency_accuracy(&self, storage_type: StorageType) -> bool {
        let accuracy_threshold = match storage_type {
            StorageType::Ssd => Duration::from_millis(1), // ±1ms for SSD
            StorageType::Hdd => Duration::from_millis(3), // ±3ms for HDD
            StorageType::Nvme => Duration::from_micros(500), // ±0.5ms for NVMe
        };

        // Check if average latency is within expected range for storage type
        let avg_within_range = match storage_type {
            StorageType::Nvme => self.avg <= Duration::from_millis(1),
            StorageType::Ssd => self.avg <= Duration::from_millis(10),
            StorageType::Hdd => self.avg <= Duration::from_millis(50),
        };

        // Check if min/max spread is reasonable (not too wide)
        let spread = self.max.saturating_sub(self.min);
        let spread_reasonable = spread <= accuracy_threshold * 10; // Allow wider spread

        avg_within_range && spread_reasonable
    }

    /// Get the 95th percentile latency
    pub fn p95(&self) -> Duration {
        self.percentiles.get(&95).copied().unwrap_or(self.max)
    }

    /// Get the 99th percentile latency
    pub fn p99(&self) -> Duration {
        self.percentiles.get(&99).copied().unwrap_or(self.max)
    }

    /// Create latency statistics from a list of samples
    pub fn from_samples(samples: &[Duration]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }

        let mut sorted = samples.to_vec();
        sorted.sort();
        let min = sorted[0];
        let max = sorted[sorted.len() - 1];
        let avg_nanos: u128 =
            sorted.iter().map(|d| d.as_nanos()).sum::<u128>() / sorted.len() as u128;
        let avg = Duration::from_nanos(avg_nanos as u64);

        let mut percentiles = HashMap::new();
        percentiles.insert(50, sorted[sorted.len() * 50 / 100]);
        percentiles.insert(95, sorted[sorted.len() * 95 / 100]);
        percentiles.insert(99, sorted[sorted.len() * 99 / 100]);

        Self {
            min,
            avg,
            max,
            percentiles,
        }
    }
}

impl Default for LatencyStats {
    fn default() -> Self {
        Self {
            min: Duration::default(),
            avg: Duration::default(),
            max: Duration::default(),
            percentiles: HashMap::new(),
        }
    }
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            os: detect_os(),
            cpu: detect_cpu(),
            memory_total: detect_memory_total(),
            memory_available: detect_memory_available(),
            storage_info: StorageInfo::detect_default(),
        }
    }
}

impl SystemInfo {
    /// Create system info by detecting current system
    pub fn detect() -> Self {
        Self::default()
    }
}

impl StorageInfo {
    /// Detect storage info for the default/current directory
    pub fn detect_default() -> Self {
        Self {
            device: "Unknown".to_string(),
            filesystem: "Unknown".to_string(),
            total_space: 0,
            available_space: 0,
        }
    }

    /// Detect storage info for a specific path
    pub fn detect_for_path(_path: &std::path::Path) -> Self {
        // TODO: Implement platform-specific storage detection
        Self::detect_default()
    }
}

/// Storage type enumeration for accuracy validation
#[derive(Debug, Clone, Copy)]
pub enum StorageType {
    /// Solid State Drive (SATA)
    Ssd,
    /// Hard Disk Drive
    Hdd,
    /// NVMe SSD
    Nvme,
}

impl StorageType {
    /// Infer storage type from performance characteristics
    pub fn infer_from_performance(throughput_mbps: f64, avg_latency: Duration) -> Self {
        let latency_ms = avg_latency.as_secs_f64() * 1000.0;

        if throughput_mbps > 1000.0 && latency_ms < 1.0 {
            StorageType::Nvme
        } else if throughput_mbps > 100.0 && latency_ms < 10.0 {
            StorageType::Ssd
        } else {
            StorageType::Hdd
        }
    }
}

// Helper functions for system detection
fn detect_os() -> String {
    format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)
}

fn detect_cpu() -> String {
    // TODO: Implement CPU detection
    "Unknown CPU".to_string()
}

fn detect_memory_total() -> u64 {
    // TODO: Implement memory detection
    0
}

fn detect_memory_available() -> u64 {
    // TODO: Implement memory detection
    0
}

// Custom serde modules for Duration serialization
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_nanos().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let nanos = u128::deserialize(deserializer)?;
        Ok(Duration::from_nanos(nanos as u64))
    }
}

mod percentiles_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;
    use std::time::Duration;

    pub fn serialize<S>(
        percentiles: &HashMap<u8, Duration>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let nanos_map: HashMap<u8, u128> = percentiles
            .iter()
            .map(|(&k, &v)| (k, v.as_nanos()))
            .collect();
        nanos_map.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<u8, Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let nanos_map = HashMap::<u8, u128>::deserialize(deserializer)?;
        let duration_map = nanos_map
            .into_iter()
            .map(|(k, v)| (k, Duration::from_nanos(v as u64)))
            .collect();
        Ok(duration_map)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BenchmarkConfig, BenchmarkMode};
    use std::time::Duration;

    fn create_test_config() -> BenchmarkConfig {
        BenchmarkConfig::sequential_write()
    }

    fn create_test_latency_stats() -> LatencyStats {
        let mut percentiles = HashMap::new();
        percentiles.insert(50, Duration::from_millis(5));
        percentiles.insert(95, Duration::from_millis(15));
        percentiles.insert(99, Duration::from_millis(25));

        LatencyStats {
            min: Duration::from_millis(1),
            avg: Duration::from_millis(5),
            max: Duration::from_millis(30),
            percentiles,
        }
    }

    fn create_test_performance_metrics() -> PerformanceMetrics {
        PerformanceMetrics::new(
            1024 * 1024 * 1024, // 1 GiB
            Duration::from_secs(10),
            create_test_latency_stats(),
        )
    }

    fn create_test_system_info() -> SystemInfo {
        SystemInfo {
            os: "Linux x86_64".to_string(),
            cpu: "Intel Core i7-9700K".to_string(),
            memory_total: 16 * 1024 * 1024 * 1024,    // 16 GiB
            memory_available: 8 * 1024 * 1024 * 1024, // 8 GiB
            storage_info: StorageInfo {
                device: "/dev/nvme0n1".to_string(),
                filesystem: "ext4".to_string(),
                total_space: 1024 * 1024 * 1024 * 1024, // 1 TiB
                available_space: 512 * 1024 * 1024 * 1024, // 512 GiB
            },
        }
    }

    #[test]
    fn test_benchmark_result_creation() {
        let config = create_test_config();
        let metrics = create_test_performance_metrics();
        let system_info = create_test_system_info();

        let result =
            BenchmarkResult::with_system_info(config.clone(), metrics.clone(), system_info.clone());

        assert_eq!(result.config.mode.description(), config.mode.description());
        assert_eq!(result.metrics.bytes_processed, metrics.bytes_processed);
        assert_eq!(result.system_info.os, system_info.os);
        assert!(result.timestamp <= Utc::now());
    }

    #[test]
    fn test_benchmark_result_summary() {
        let result = BenchmarkResult::with_system_info(
            create_test_config(),
            create_test_performance_metrics(),
            create_test_system_info(),
        );

        let summary = result.summary();
        assert!(summary.contains("Sequential Write"));
        assert!(summary.contains("MB/s"));
        assert!(summary.contains("IOPS"));
        assert!(summary.contains("ms avg latency"));
    }

    #[test]
    fn test_performance_metrics_calculation() {
        let bytes_processed = 1024 * 1024 * 1024; // 1 GiB
        let elapsed_time = Duration::from_secs(10);
        let latency = create_test_latency_stats();

        let metrics = PerformanceMetrics::new(bytes_processed, elapsed_time, latency);

        // Throughput should be ~102.4 MB/s (1 GiB / 10 seconds)
        assert!((metrics.throughput_mbps - 102.4).abs() < 0.1);

        // IOPS should be 1 / avg_latency (1 / 0.005s = 200 IOPS)
        assert!((metrics.iops - 200.0).abs() < 1.0);

        assert_eq!(metrics.bytes_processed, bytes_processed);
        assert_eq!(metrics.elapsed_time, elapsed_time);
    }

    #[test]
    fn test_performance_metrics_zero_time() {
        let metrics =
            PerformanceMetrics::new(1024, Duration::from_secs(0), create_test_latency_stats());

        assert_eq!(metrics.throughput_mbps, 0.0);
        assert_eq!(metrics.iops, 0.0);
    }

    #[test]
    fn test_performance_metrics_efficiency_ratio() {
        let metrics = create_test_performance_metrics();

        assert!((metrics.efficiency_ratio(1) - metrics.throughput_mbps).abs() < 0.001);
        assert!((metrics.efficiency_ratio(4) - metrics.throughput_mbps / 4.0).abs() < 0.001);
        assert_eq!(metrics.efficiency_ratio(0), 0.0);
    }

    #[test]
    fn test_latency_stats_creation() {
        let latency = LatencyStats::new(
            Duration::from_millis(1),
            Duration::from_millis(5),
            Duration::from_millis(30),
        );

        assert_eq!(latency.min, Duration::from_millis(1));
        assert_eq!(latency.avg, Duration::from_millis(5));
        assert_eq!(latency.max, Duration::from_millis(30));
        assert!(latency.percentiles.contains_key(&50));
        assert!(latency.percentiles.contains_key(&95));
        assert!(latency.percentiles.contains_key(&99));
    }

    #[test]
    fn test_latency_stats_percentiles() {
        let latency = create_test_latency_stats();

        assert_eq!(latency.p95(), Duration::from_millis(15));
        assert_eq!(latency.p99(), Duration::from_millis(25));

        // Test fallback to max when percentile doesn't exist
        let simple_latency = LatencyStats::new(
            Duration::from_millis(1),
            Duration::from_millis(5),
            Duration::from_millis(30),
        );
        assert_eq!(simple_latency.p95(), Duration::from_millis(30));
    }

    #[test]
    fn test_latency_stats_accuracy_validation() {
        let ssd_latency = LatencyStats::new(
            Duration::from_micros(100),
            Duration::from_micros(500),
            Duration::from_millis(1),
        );
        assert!(ssd_latency.meets_latency_accuracy(StorageType::Ssd));

        let hdd_latency = LatencyStats::new(
            Duration::from_millis(5),
            Duration::from_millis(10),
            Duration::from_millis(15),
        );
        assert!(hdd_latency.meets_latency_accuracy(StorageType::Hdd));

        let bad_latency = LatencyStats::new(
            Duration::from_millis(1),
            Duration::from_millis(50),
            Duration::from_secs(1),
        );
        assert!(!bad_latency.meets_latency_accuracy(StorageType::Ssd));
    }

    #[test]
    fn test_storage_type_inference() {
        // NVMe characteristics
        let nvme_type = StorageType::infer_from_performance(2000.0, Duration::from_micros(500));
        assert!(matches!(nvme_type, StorageType::Nvme));

        // SSD characteristics
        let ssd_type = StorageType::infer_from_performance(500.0, Duration::from_millis(2));
        assert!(matches!(ssd_type, StorageType::Ssd));

        // HDD characteristics
        let hdd_type = StorageType::infer_from_performance(100.0, Duration::from_millis(15));
        assert!(matches!(hdd_type, StorageType::Hdd));
    }

    #[test]
    fn test_benchmark_result_accuracy_validation() {
        let base_result = BenchmarkResult::with_system_info(
            create_test_config(),
            create_test_performance_metrics(),
            create_test_system_info(),
        );

        // Test with insufficient data
        assert!(base_result.meets_accuracy_requirements(&[]));
        assert!(base_result.meets_accuracy_requirements(&[base_result.clone()]));

        // Test with consistent results (should pass)
        let consistent_metrics = PerformanceMetrics::new(
            1024 * 1024 * 1024,
            Duration::from_secs(10),
            create_test_latency_stats(),
        );
        let consistent_result = BenchmarkResult::with_system_info(
            create_test_config(),
            consistent_metrics,
            create_test_system_info(),
        );

        assert!(base_result
            .meets_accuracy_requirements(&[consistent_result.clone(), consistent_result.clone(),]));

        // Test with inconsistent results (should fail)
        let inconsistent_metrics = PerformanceMetrics::new(
            1024 * 1024 * 1024,
            Duration::from_secs(5), // Much faster, will cause high deviation
            create_test_latency_stats(),
        );
        let inconsistent_result = BenchmarkResult::with_system_info(
            create_test_config(),
            inconsistent_metrics,
            create_test_system_info(),
        );

        assert!(
            !base_result.meets_accuracy_requirements(&[consistent_result, inconsistent_result,])
        );
    }

    #[test]
    fn test_system_info_detection() {
        let system_info = SystemInfo::detect();
        assert!(!system_info.os.is_empty());
        assert!(!system_info.cpu.is_empty());
    }

    #[test]
    fn test_storage_info_detection() {
        let storage_info = StorageInfo::detect_default();
        assert_eq!(storage_info.device, "Unknown");
        assert_eq!(storage_info.filesystem, "Unknown");
    }

    #[test]
    fn test_serde_serialization() {
        let result = BenchmarkResult::with_system_info(
            create_test_config(),
            create_test_performance_metrics(),
            create_test_system_info(),
        );

        // Test JSON serialization
        let json = serde_json::to_string(&result).expect("Failed to serialize to JSON");
        assert!(!json.is_empty());

        // Test JSON deserialization
        let deserialized: BenchmarkResult =
            serde_json::from_str(&json).expect("Failed to deserialize from JSON");

        assert_eq!(result.config.file_size, deserialized.config.file_size);
        assert_eq!(
            result.metrics.bytes_processed,
            deserialized.metrics.bytes_processed
        );
        assert_eq!(result.system_info.os, deserialized.system_info.os);
        assert_eq!(result.timestamp, deserialized.timestamp);
    }

    #[test]
    fn test_duration_serde() {
        let original_duration = Duration::from_nanos(123456789);
        let serialized = serde_json::to_string(&original_duration).unwrap();
        let deserialized: Duration = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original_duration, deserialized);
    }

    #[test]
    fn test_percentiles_serde() {
        let mut percentiles = HashMap::new();
        percentiles.insert(50, Duration::from_millis(5));
        percentiles.insert(95, Duration::from_millis(15));
        percentiles.insert(99, Duration::from_millis(25));

        let serialized = serde_json::to_string(&percentiles).unwrap();
        let deserialized: HashMap<u8, Duration> = serde_json::from_str(&serialized).unwrap();

        assert_eq!(percentiles.len(), deserialized.len());
        for (&key, &value) in &percentiles {
            assert_eq!(Some(&value), deserialized.get(&key));
        }
    }

    #[test]
    fn test_benchmark_modes_in_results() {
        let modes = vec![
            BenchmarkMode::SequentialWrite,
            BenchmarkMode::SequentialRead,
            BenchmarkMode::RandomReadWrite,
            BenchmarkMode::Mixed { read_ratio: 0.7 },
        ];

        for mode in modes {
            let mut config = create_test_config();
            config.mode = mode;

            let result = BenchmarkResult::with_system_info(
                config,
                create_test_performance_metrics(),
                create_test_system_info(),
            );

            let summary = result.summary();
            assert!(summary.contains(result.config.mode.description()));
        }
    }

    #[test]
    fn test_latency_stats_from_samples() {
        let samples = vec![
            Duration::from_micros(100),
            Duration::from_micros(200),
            Duration::from_micros(300),
        ];

        let stats = LatencyStats::from_samples(&samples);
        assert_eq!(stats.min, Duration::from_micros(100));
        assert_eq!(stats.max, Duration::from_micros(300));
        assert_eq!(stats.avg, Duration::from_micros(200));
        assert_eq!(stats.p95(), Duration::from_micros(300));
    }

    #[test]
    fn test_performance_metrics_validate_throughput() {
        let metrics = PerformanceMetrics::new(
            2 * 1024 * 1024,
            Duration::from_secs(2),
            create_test_latency_stats(),
        );
        assert!(metrics.validate_throughput());
    }
}
