use std::process::{Command, Stdio};
use std::time::Duration;
use std::thread;

fn main() {
    println!("Testing DIORB application startup...");
    
    let mut child = Command::new("cargo")
        .args(&["run"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start application");
    
    // Wait a bit to see if it starts properly
    thread::sleep(Duration::from_secs(2));
    
    // Check if the process is still running
    match child.try_wait() {
        Ok(Some(status)) => {
            println!("Application exited with status: {}", status);
            
            let output = child.wait_with_output().unwrap();
            println!("STDOUT:");
            println!("{}", String::from_utf8_lossy(&output.stdout));
            println!("STDERR:");
            println!("{}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(None) => {
            println!("Application is still running - this is good!");
            println!("Terminating test...");
            child.kill().expect("Failed to kill child process");
        }
        Err(e) => {
            println!("Error checking process status: {}", e);
        }
    }
}