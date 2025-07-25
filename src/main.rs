use diorb::config::BenchmarkConfig;
use diorb::simple::{ask_config, detect_disks, run_speedtest};
use diorb::Result;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Detect disks and ask user to choose one
    let disks = detect_disks();
    println!("Available disks:");
    for (i, d) in disks.iter().enumerate() {
        println!("{}: {}", i + 1, d.display());
    }
    println!("Enter disk number:");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let idx = input.trim().parse::<usize>().unwrap_or(1);
    let disk = disks
        .get(idx - 1)
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));

    // Use default config then optionally override
    let mut config = BenchmarkConfig::sequential_write().with_disk_path(disk);
    println!("Press Enter to accept default config or type 'c' to change:");
    input.clear();
    std::io::stdin().read_line(&mut input)?;
    if input.trim().eq_ignore_ascii_case("c") {
        config = ask_config(config)?;
    }

    // Run speed test
    let result = run_speedtest(config.clone()).await?;
    println!(
        "\nCompleted {} bytes in {:.2}s",
        result.metrics.bytes_processed,
        result.metrics.elapsed_time.as_secs_f64()
    );
    println!("Throughput: {:.2} MB/s", result.metrics.throughput_mbps);
    println!("IOPS: {:.0}", result.metrics.iops);
    Ok(())
}
