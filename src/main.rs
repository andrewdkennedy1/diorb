use diorb::{app::App, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Create and initialize the application
    let mut app = App::new()?;
    app.init()?;
    
    // Run the application
    if let Err(e) = app.run().await {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
    
    Ok(())
}
