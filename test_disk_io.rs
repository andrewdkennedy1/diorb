use std::path::PathBuf;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing disk I/O operations...");
    
    // Test basic file creation and writing
    let test_path = PathBuf::from("test_file.dat");
    let test_data = vec![0u8; 1024 * 1024]; // 1MB of data
    
    println!("Writing 1MB test file...");
    let start = Instant::now();
    std::fs::write(&test_path, &test_data)?;
    let write_time = start.elapsed();
    println!("Write completed in {:?}", write_time);
    
    println!("Reading 1MB test file...");
    let start = Instant::now();
    let read_data = std::fs::read(&test_path)?;
    let read_time = start.elapsed();
    println!("Read completed in {:?}", read_time);
    
    println!("Data integrity check: {}", read_data == test_data);
    
    // Calculate throughput
    let write_mbps = 1.0 / write_time.as_secs_f64();
    let read_mbps = 1.0 / read_time.as_secs_f64();
    
    println!("Write throughput: {:.2} MB/s", write_mbps);
    println!("Read throughput: {:.2} MB/s", read_mbps);
    
    // Clean up
    std::fs::remove_file(&test_path)?;
    println!("Test completed successfully!");
    
    Ok(())
}