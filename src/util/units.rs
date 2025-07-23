//! Units formatting and conversion utilities
//! 
//! Provides functions for human-readable formatting of sizes, durations,
//! and performance metrics like throughput and IOPS.

use std::time::Duration;

/// Format bytes into human-readable size with appropriate units
/// 
/// # Examples
/// ```
/// use diorb::util::units::format_bytes;
/// 
/// assert_eq!(format_bytes(1024), "1.0 KiB");
/// assert_eq!(format_bytes(1048576), "1.0 MiB");
/// assert_eq!(format_bytes(1073741824), "1.0 GiB");
/// ```
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    const THRESHOLD: f64 = 1024.0;
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= THRESHOLD && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Parse human-readable size string into bytes
/// 
/// Supports units: B, KB, MB, GB, TB, KiB, MiB, GiB, TiB
/// 
/// # Examples
/// ```
/// use diorb::util::units::parse_bytes;
/// 
/// assert_eq!(parse_bytes("1 KiB").unwrap(), 1024);
/// assert_eq!(parse_bytes("1.5 MiB").unwrap(), 1572864);
/// assert_eq!(parse_bytes("2 GB").unwrap(), 2000000000);
/// ```
pub fn parse_bytes(input: &str) -> Result<u64, String> {
    let input = input.trim();
    
    // Find the last space or digit-letter boundary
    let (number_part, unit_part) = if let Some(space_pos) = input.rfind(' ') {
        (&input[..space_pos], &input[space_pos + 1..])
    } else {
        // Find where digits end and letters begin
        let mut split_pos = input.len();
        for (i, c) in input.char_indices() {
            if c.is_alphabetic() {
                split_pos = i;
                break;
            }
        }
        (&input[..split_pos], &input[split_pos..])
    };
    
    let number: f64 = number_part.parse()
        .map_err(|_| format!("Invalid number: {}", number_part))?;
    
    if number < 0.0 {
        return Err("Size cannot be negative".to_string());
    }
    
    let multiplier = match unit_part.to_uppercase().as_str() {
        "" | "B" => 1u64,
        "KB" => 1_000u64,
        "MB" => 1_000_000u64,
        "GB" => 1_000_000_000u64,
        "TB" => 1_000_000_000_000u64,
        "KIB" => 1_024u64,
        "MIB" => 1_048_576u64,
        "GIB" => 1_073_741_824u64,
        "TIB" => 1_099_511_627_776u64,
        _ => return Err(format!("Unknown unit: {}", unit_part)),
    };
    
    Ok((number * multiplier as f64) as u64)
}

/// Format duration into human-readable string
/// 
/// # Examples
/// ```
/// use std::time::Duration;
/// use diorb::util::units::format_duration;
/// 
/// assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
/// assert_eq!(format_duration(Duration::from_millis(1500)), "1.50s");
/// ```
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let millis = duration.subsec_millis();
    
    if total_secs >= 3600 {
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if total_secs >= 60 {
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{}m {}s", minutes, seconds)
    } else if total_secs > 0 {
        if millis > 0 {
            format!("{}.{:02}s", total_secs, millis / 10)
        } else {
            format!("{}s", total_secs)
        }
    } else {
        format!("{}ms", millis)
    }
}

/// Parse duration string into Duration
/// 
/// Supports formats like: "30s", "1m 30s", "1h 30m", "500ms"
/// 
/// # Examples
/// ```
/// use std::time::Duration;
/// use diorb::util::units::parse_duration;
/// 
/// assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
/// assert_eq!(parse_duration("1m 30s").unwrap(), Duration::from_secs(90));
/// ```
pub fn parse_duration(input: &str) -> Result<Duration, String> {
    let input = input.trim().to_lowercase();
    let mut total_secs = 0u64;
    let mut total_millis = 0u64;
    
    // Split by spaces and parse each component
    let parts: Vec<&str> = input.split_whitespace().collect();
    
    for part in parts {
        if part.ends_with("ms") {
            let num_str = &part[..part.len() - 2];
            let millis: u64 = num_str.parse()
                .map_err(|_| format!("Invalid milliseconds: {}", num_str))?;
            total_millis += millis;
        } else if part.ends_with('s') {
            let num_str = &part[..part.len() - 1];
            let secs: f64 = num_str.parse()
                .map_err(|_| format!("Invalid seconds: {}", num_str))?;
            total_secs += secs as u64;
            total_millis += (secs.fract() * 1000.0) as u64;
        } else if part.ends_with('m') {
            let num_str = &part[..part.len() - 1];
            let mins: u64 = num_str.parse()
                .map_err(|_| format!("Invalid minutes: {}", num_str))?;
            total_secs += mins * 60;
        } else if part.ends_with('h') {
            let num_str = &part[..part.len() - 1];
            let hours: u64 = num_str.parse()
                .map_err(|_| format!("Invalid hours: {}", num_str))?;
            total_secs += hours * 3600;
        } else {
            return Err(format!("Unknown duration format: {}", part));
        }
    }
    
    Ok(Duration::from_millis(total_secs * 1000 + total_millis))
}

/// Calculate throughput in MB/s from bytes and duration
/// 
/// # Examples
/// ```
/// use std::time::Duration;
/// use diorb::util::units::calculate_throughput_mbps;
/// 
/// let throughput = calculate_throughput_mbps(1048576, Duration::from_secs(1));
/// assert!((throughput - 1.0).abs() < 0.01);
/// ```
pub fn calculate_throughput_mbps(bytes: u64, duration: Duration) -> f64 {
    if duration.is_zero() {
        return 0.0;
    }
    
    let duration_secs = duration.as_secs_f64();
    let megabytes = bytes as f64 / 1_048_576.0; // 1 MiB = 1,048,576 bytes
    megabytes / duration_secs
}

/// Calculate IOPS (Input/Output Operations Per Second)
/// 
/// # Examples
/// ```
/// use std::time::Duration;
/// use diorb::util::units::calculate_iops;
/// 
/// let iops = calculate_iops(1000, Duration::from_secs(1));
/// assert!((iops - 1000.0).abs() < 0.01);
/// ```
pub fn calculate_iops(operations: u64, duration: Duration) -> f64 {
    if duration.is_zero() {
        return 0.0;
    }
    
    let duration_secs = duration.as_secs_f64();
    operations as f64 / duration_secs
}

/// Format throughput value with appropriate units
/// 
/// # Examples
/// ```
/// use diorb::util::units::format_throughput;
/// 
/// assert_eq!(format_throughput(1024.0), "1.0 GiB/s");
/// assert_eq!(format_throughput(1.5), "1.5 MiB/s");
/// ```
pub fn format_throughput(mbps: f64) -> String {
    if mbps >= 1024.0 {
        format!("{:.1} GiB/s", mbps / 1024.0)
    } else if mbps >= 1.0 {
        format!("{:.1} MiB/s", mbps)
    } else if mbps >= 0.001 {
        format!("{:.1} KiB/s", mbps * 1024.0)
    } else {
        format!("{:.3} MiB/s", mbps)
    }
}

/// Format IOPS value with appropriate units
/// 
/// # Examples
/// ```
/// use diorb::util::units::format_iops;
/// 
/// assert_eq!(format_iops(1500.0), "1.5K IOPS");
/// assert_eq!(format_iops(2500000.0), "2.5M IOPS");
/// ```
pub fn format_iops(iops: f64) -> String {
    if iops >= 1_000_000.0 {
        format!("{:.1}M IOPS", iops / 1_000_000.0)
    } else if iops >= 1_000.0 {
        format!("{:.1}K IOPS", iops / 1_000.0)
    } else {
        format!("{:.0} IOPS", iops)
    }
}

/// Format latency duration with appropriate precision
/// 
/// # Examples
/// ```
/// use std::time::Duration;
/// use diorb::util::units::format_latency;
/// 
/// assert_eq!(format_latency(Duration::from_millis(5)), "5.00ms");
/// assert_eq!(format_latency(Duration::from_micros(500)), "0.50ms");
/// ```
pub fn format_latency(duration: Duration) -> String {
    let micros = duration.as_micros();
    
    if micros >= 1000 {
        let millis = micros as f64 / 1000.0;
        format!("{:.2}ms", millis)
    } else {
        format!("{}μs", micros)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1536), "1.5 KiB");
        assert_eq!(format_bytes(1048576), "1.0 MiB");
        assert_eq!(format_bytes(1073741824), "1.0 GiB");
        assert_eq!(format_bytes(1099511627776), "1.0 TiB");
    }

    #[test]
    fn test_parse_bytes() {
        assert_eq!(parse_bytes("1024").unwrap(), 1024);
        assert_eq!(parse_bytes("1 KiB").unwrap(), 1024);
        assert_eq!(parse_bytes("1.5 MiB").unwrap(), 1572864);
        assert_eq!(parse_bytes("2 GB").unwrap(), 2000000000);
        assert_eq!(parse_bytes("1GiB").unwrap(), 1073741824);
        
        assert!(parse_bytes("invalid").is_err());
        assert!(parse_bytes("-1 MB").is_err());
        assert!(parse_bytes("1 XB").is_err());
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.50s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m 1s");
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("1m 30s").unwrap(), Duration::from_secs(90));
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_duration("1.5s").unwrap(), Duration::from_millis(1500));
        
        assert!(parse_duration("invalid").is_err());
        assert!(parse_duration("1x").is_err());
    }

    #[test]
    fn test_calculate_throughput_mbps() {
        let throughput = calculate_throughput_mbps(1048576, Duration::from_secs(1));
        assert!((throughput - 1.0).abs() < 0.01);
        
        let throughput = calculate_throughput_mbps(2097152, Duration::from_secs(2));
        assert!((throughput - 1.0).abs() < 0.01);
        
        assert_eq!(calculate_throughput_mbps(1000, Duration::ZERO), 0.0);
    }

    #[test]
    fn test_calculate_iops() {
        let iops = calculate_iops(1000, Duration::from_secs(1));
        assert!((iops - 1000.0).abs() < 0.01);
        
        let iops = calculate_iops(500, Duration::from_millis(500));
        assert!((iops - 1000.0).abs() < 0.01);
        
        assert_eq!(calculate_iops(1000, Duration::ZERO), 0.0);
    }

    #[test]
    fn test_format_throughput() {
        assert_eq!(format_throughput(1.5), "1.5 MiB/s");
        assert_eq!(format_throughput(1024.0), "1.0 GiB/s");
        assert_eq!(format_throughput(0.5), "512.0 KiB/s");
        assert_eq!(format_throughput(0.0001), "0.000 MiB/s");
    }

    #[test]
    fn test_format_iops() {
        assert_eq!(format_iops(500.0), "500 IOPS");
        assert_eq!(format_iops(1500.0), "1.5K IOPS");
        assert_eq!(format_iops(2500000.0), "2.5M IOPS");
    }

    #[test]
    fn test_format_latency() {
        assert_eq!(format_latency(Duration::from_millis(5)), "5.00ms");
        assert_eq!(format_latency(Duration::from_micros(500)), "500μs");
        assert_eq!(format_latency(Duration::from_micros(1500)), "1.50ms");
        assert_eq!(format_latency(Duration::from_micros(50)), "50μs");
    }
}