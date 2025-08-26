mod agent;
mod config;
mod context;
mod interactive;
mod mcp_client;
mod gitlab_tools;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Ken - GitLab Assistant");
    println!("Starting interactive mode...\n");
    
    // Start interactive session directly
    let mut session = interactive::KenSession::new().await?;
    session.start_interactive().await?;
    
    Ok(())
}
