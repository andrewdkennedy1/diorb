use diorb::{app::App, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("DIORB Debug - Starting application...");
    
    // Create and initialize the application
    let mut app = App::new()?;
    println!("DIORB Debug - App created successfully");
    
    app.init()?;
    println!("DIORB Debug - App initialized successfully");
    
    // Run the application
    println!("DIORB Debug - Starting main loop...");
    if let Err(e) = app.run().await {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
    
    println!("DIORB Debug - Application finished");
    Ok(())
}