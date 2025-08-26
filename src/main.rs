mod config;
mod interactive;

use anyhow::Result;
use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Ken - GitLab Assistant");
    println!("Starting interactive mode...\n");
    
    // Start interactive session directly
    let mut session = interactive::KenSession::new().await?;
    session.start_interactive().await?;
    
    Ok(())
}
