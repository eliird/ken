mod agent;
mod cli;
mod config;
mod tools;

use agent::KenAgent;
use anyhow::Result;
use clap::Parser;
use cli::{AuthCommands, Cli, Commands, ProjectCommands};
use config::Config;
use rig::completion::Chat;
use serde_json;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Auth { subcommand } => match subcommand {
            AuthCommands::Login => {
                println!("🔐 Ken - GitLab Authentication Setup");
                println!();
                
                let config = Config::prompt_for_login()?;
                
                // Verify the credentials work
                println!("\nVerifying credentials...");
                config.verify().await?;
                
                // Save the config
                config.save()?;
                println!("✅ Configuration saved successfully!");
                println!("\nYou can now use Ken to manage GitLab issues.");
            }
            
            AuthCommands::Status => {
                match Config::load() {
                    Ok(config) => {
                        println!("✅ Authenticated to: {}", config.gitlab_url);
                        if let Some(ref project) = config.default_project_id {
                            println!("Default project: {}", project);
                        }
                        
                        // Verify the token is still valid
                        if config.verify().await.is_ok() {
                            println!("Token is valid and working.");
                        } else {
                            println!("⚠️  Token may have expired or been revoked.");
                        }
                    }
                    Err(_) => {
                        println!("❌ Not authenticated. Run 'ken auth login' to set up GitLab access.");
                    }
                }
            }
            
            AuthCommands::Logout => {
                let config_path = Config::config_path()?;
                if config_path.exists() {
                    std::fs::remove_file(config_path)?;
                    println!("✅ Logged out successfully. Credentials removed.");
                } else {
                    println!("Not currently logged in.");
                }
            }
        },
        
        Commands::Issue { description } => {
            // Load config first
            let _config = Config::load()?;
            
            println!("Creating issue from: {}", description);
            
            // TODO: Use agent to process the description and create issue
            let agent = KenAgent::default();
            match agent.chat(&description, Vec::new()).await {
                Ok(response) => {
                    println!("{}", response);
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                }
            }
        }
        
        Commands::Summarize { issue_id } => {
            let _config = Config::load()?;
            println!("Summarizing issue: {}", issue_id);
            // TODO: Implement summarization
        }
        
        Commands::Suggest { issue_id } => {
            let _config = Config::load()?;
            println!("Suggesting assignee for issue: {}", issue_id);
            // TODO: Implement suggestion
        }
        
        Commands::Workload { username } => {
            let _config = Config::load()?;
            println!("Checking workload for: {}", username);
            // TODO: Implement workload check
        }
        
        Commands::Query { question, project } => {
            // Load config first
            let mut config = Config::load()?;
            
            // Override project if specified
            if let Some(proj) = project {
                config.default_project_id = Some(proj.clone());
                println!("📁 Using project: {}", proj);
            } else if let Some(ref proj) = config.default_project_id {
                println!("📁 Using default project: {}", proj);
            } else {
                eprintln!("❌ No project specified. Use --project flag or set default with 'ken project set'");
                return Ok(());
            }
            
            println!("🔍 Processing query: {}", question);
            println!();
            
            // Create agent with GitLab tools
            let agent = KenAgent::with_gitlab_tools(&config);
            
            // Process the query
            match agent.chat(&question, Vec::new()).await {
                Ok(response) => {
                    println!("{}", response);
                }
                Err(err) => {
                    eprintln!("❌ Error: {}", err);
                }
            }
        }
        
        Commands::Project { subcommand } => match subcommand {
            ProjectCommands::List { search, mine } => {
                let config = Config::load()?;
                println!("📋 Fetching projects from GitLab...");
                
                let mut url = format!("{}/api/v4/projects", config.gitlab_url);
                let mut params = vec!["simple=true".to_string(), "per_page=50".to_string()];
                
                if mine {
                    params.push("owned=true".to_string());
                }
                
                if let Some(search_term) = search {
                    params.push(format!("search={}", urlencoding::encode(&search_term)));
                }
                
                if !params.is_empty() {
                    url.push_str("?");
                    url.push_str(&params.join("&"));
                }
                
                let client = reqwest::Client::new();
                let response = client
                    .get(&url)
                    .header("PRIVATE-TOKEN", &config.api_token)
                    .send()
                    .await?;
                
                if response.status().is_success() {
                    let projects: Vec<serde_json::Value> = response.json().await?;
                    
                    if projects.is_empty() {
                        println!("No projects found.");
                    } else {
                        println!("\n📂 Available Projects:");
                        println!("─────────────────────");
                        for project in projects.iter().take(20) {
                            let name = project.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                            let id = project.get("id").and_then(|i| i.as_i64()).unwrap_or(0);
                            let path_with_namespace = project.get("path_with_namespace")
                                .and_then(|p| p.as_str())
                                .unwrap_or("");
                            
                            println!("  • {} (ID: {}, Path: {})", name, id, path_with_namespace);
                        }
                        
                        if projects.len() > 20 {
                            println!("\n  ... and {} more projects", projects.len() - 20);
                        }
                        
                        println!("\n💡 Tip: Use 'ken project set <project_id>' to set a default project");
                        println!("   You can use either the numeric ID or the path (namespace/project)");
                    }
                } else {
                    eprintln!("❌ Failed to fetch projects: {}", response.status());
                }
            }
            
            ProjectCommands::Set { project_id } => {
                let mut config = Config::load()?;
                config.default_project_id = Some(project_id.clone());
                config.save()?;
                println!("✅ Default project set to: {}", project_id);
            }
            
            ProjectCommands::Current => {
                let config = Config::load()?;
                match config.default_project_id {
                    Some(project) => println!("📁 Current default project: {}", project),
                    None => println!("❌ No default project set. Use 'ken project set <project_id>' to set one."),
                }
            }
        }
    }

    Ok(())
}
