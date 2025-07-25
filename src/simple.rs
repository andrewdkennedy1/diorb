use std::io::{self, Write};
use std::path::PathBuf;

use crate::bench::sequential::{ProgressUpdate, SequentialBenchmark};
use crate::config::{BenchmarkConfig, BenchmarkMode};
use crate::models::BenchmarkResult;
use crate::Result;

/// Detect available disks on the system.
/// This is a simplified version that checks common locations.
pub fn detect_disks() -> Vec<PathBuf> {
    let mut disks = Vec::new();

    #[cfg(windows)]
    {
        unsafe {
            extern "system" {
                fn GetLogicalDrives() -> u32;
            }
            let mask = GetLogicalDrives();
            for i in 0..26 {
                if (mask & (1 << i)) != 0 {
                    let drive = (b'A' + i as u8) as char;
                    let path = PathBuf::from(format!("{}:\\", drive));
                    if path.exists() {
                        disks.push(path);
                    }
                }
            }
        }
        if disks.is_empty() {
            for drive in 'C'..='Z' {
                let path = PathBuf::from(format!("{}:\\", drive));
                if path.exists() {
                    disks.push(path);
                }
            }
        }
    }

    #[cfg(unix)]
    {
        let candidates = ["/", "/mnt", "/media", "/tmp", "/var", "/home"];
        for p in &candidates {
            let path = PathBuf::from(p);
            if path.exists() && path.is_dir() {
                disks.push(path);
            }
        }
        if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
            for line in mounts.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let mnt = PathBuf::from(parts[1]);
                    if mnt.exists() && mnt.is_dir() && !disks.contains(&mnt) {
                        disks.push(mnt);
                    }
                }
            }
        }
    }

    if let Ok(cur) = std::env::current_dir() {
        if !disks.contains(&cur) {
            disks.insert(0, cur);
        }
    }

    if disks.is_empty() {
        disks.push(PathBuf::from("."));
    }
    disks.sort();
    disks
}

/// Prompt the user for simple configuration overrides.
pub fn ask_config(mut config: BenchmarkConfig) -> Result<BenchmarkConfig> {
    let mb = config.file_size / (1024 * 1024);
    print!("File size in MB (default {}): ", mb);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if let Ok(v) = input.trim().parse::<u64>() {
        config.file_size = v * 1024 * 1024;
    }

    let kb = config.block_size / 1024;
    input.clear();
    print!("Block size in KB (default {}): ", kb);
    io::stdout().flush()?;
    io::stdin().read_line(&mut input)?;
    if let Ok(v) = input.trim().parse::<u64>() {
        config.block_size = v * 1024;
    }

    Ok(config)
}

/// Run the sequential write speed test and stream progress.
pub async fn run_speedtest(config: BenchmarkConfig) -> Result<BenchmarkResult> {
    let benchmark = SequentialBenchmark::new(config.clone())?;
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let pb = indicatif::ProgressBar::new(config.file_size);
    pb.set_style(
        indicatif::ProgressStyle::with_template("{spinner} {bytes}/{total_bytes} ({eta}) {msg}")
            .unwrap(),
    );

    let handle = tokio::spawn(async move {
        while let Some(ProgressUpdate {
            bytes_processed,
            throughput_mbps,
            ..
        }) = rx.recv().await
        {
            pb.set_position(bytes_processed);
            pb.set_message(format!("{:.1} MB/s", throughput_mbps));
        }
        pb.finish();
    });

    let result = benchmark.run(tx).await?;
    handle.await.ok();
    Ok(result)
}
