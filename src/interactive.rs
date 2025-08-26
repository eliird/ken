use anyhow::Result;
use rustyline::Editor;
use crate::config::Config;

pub struct KenSession {
    pub config: Option<Config>,
    pub editor: Editor<(), rustyline::history::DefaultHistory>,
}

impl KenSession {
    pub async fn new() -> Result<Self> {
        let editor = Editor::new().map_err(|e| anyhow::anyhow!("Failed to create editor: {}", e))?;
        
        // Try to load existing config, but don't fail if it doesn't exist
        let config = Config::load().ok();
        
        Ok(KenSession {
            config,
            editor,
        })
    }
    
    pub async fn start_interactive(&mut self) -> Result<()> {
        // Show startup banner
        self.print_banner();
        
        loop {
            // Get user input
            let readline = self.editor.readline("Ken> ");
            
            match readline {
                Ok(line) => {
                    let trimmed = line.trim();
                    
                    // Skip empty lines
                    if trimmed.is_empty() {
                        continue;
                    }
                    
                    // Add to history
                    self.editor.add_history_entry(trimmed)
                        .map_err(|e| anyhow::anyhow!("Failed to add history: {}", e))?;
                    
                    // Handle exit commands
                    if matches!(trimmed.to_lowercase().as_str(), "exit" | "quit" | "/exit" | "/quit") {
                        println!("👋 Goodbye!");
                        break;
                    }
                    
                    // Process the command
                    if let Err(e) = self.process_input(trimmed).await {
                        eprintln!("❌ Error: {}", e);
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    // Ctrl-C
                    println!("👋 Goodbye!");
                    break;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    // Ctrl-D
                    println!("👋 Goodbye!");
                    break;
                }
                Err(err) => {
                    eprintln!("❌ Error reading input: {}", err);
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    fn print_banner(&self) {
        if let Some(ref config) = self.config {
            println!("✅ Authenticated to: {}", config.gitlab_url);
            if let Some(ref project) = config.default_project_id {
                println!("📁 Current project: {}", project);
            } else {
                println!("❌ No default project set.");
            }
        } else {
            println!("❌ Not authenticated. Use '/login' to authenticate.");
        }
        println!("💡 Type '/help' for commands or 'exit' to quit.\n");
    }
    
    async fn process_input(&mut self, input: &str) -> Result<()> {
        if input.starts_with('/') {
            self.handle_command(input).await
        } else {
            self.handle_query(input).await
        }
    }
    
    async fn handle_command(&mut self, command: &str) -> Result<()> {
        match command {
            "/help" => {
                println!("📋 Available Commands:");
                println!("  /help           - Show this help");
                println!("  /login          - Login to GitLab");
                println!("  /logout         - Logout and remove credentials");
                println!("  /status         - Check authentication status");
                println!("  /projects       - List available projects");
                println!("  /project <id>   - Set default project");
                println!("  /current        - Show current project");
                println!("  exit            - Quit Ken");
            }
            "/login" => {
                println!("🔐 GitLab Authentication Setup");
                let new_config = Config::prompt_for_login()?;
                
                println!("🔄 Verifying credentials...");
                new_config.verify().await?;
                
                new_config.save()?;
                self.config = Some(new_config);
                println!("✅ Login successful!");
            }
            "/logout" => {
                if self.config.is_some() {
                    let config_path = Config::config_path()?;
                    if config_path.exists() {
                        std::fs::remove_file(config_path)?;
                    }
                    self.config = None;
                    println!("✅ Logged out successfully!");
                } else {
                    println!("❌ Not currently logged in.");
                }
            }
            "/status" => {
                if let Some(ref config) = self.config {
                    println!("✅ Authenticated to: {}", config.gitlab_url);
                    if let Some(ref project) = config.default_project_id {
                        println!("📁 Default project: {}", project);
                    }
                    
                    print!("🔄 Verifying token... ");
                    if config.verify().await.is_ok() {
                        println!("✅ Token is valid.");
                    } else {
                        println!("❌ Token expired or invalid.");
                    }
                } else {
                    println!("❌ Not authenticated. Use '/login' first.");
                }
            }
            "/projects" => {
                if let Some(ref config) = self.config {
                    self.list_projects(config).await?;
                } else {
                    println!("❌ Not authenticated. Use '/login' first.");
                }
            }
            "/current" => {
                if let Some(ref config) = self.config {
                    if let Some(ref project) = config.default_project_id {
                        println!("📁 Current project: {}", project);
                    } else {
                        println!("❌ No default project set.");
                    }
                } else {
                    println!("❌ Not authenticated. Use '/login' first.");
                }
            }
            _ if command.starts_with("/project ") => {
                let project_id = command[9..].trim(); // Remove "/project "
                if project_id.is_empty() {
                    println!("❌ Please specify a project ID: /project <id>");
                } else if let Some(ref mut config) = self.config {
                    config.default_project_id = Some(project_id.to_string());
                    config.save()?;
                    println!("✅ Default project set to: {}", project_id);
                } else {
                    println!("❌ Not authenticated. Use '/login' first.");
                }
            }
            _ => {
                println!("❓ Unknown command: {}. Type '/help' for available commands.", command);
            }
        }
        Ok(())
    }
    
    async fn handle_query(&self, query: &str) -> Result<()> {
        println!("🔍 Natural language queries not implemented yet: {}", query);
        Ok(())
    }

    async fn list_projects(&self, config: &Config) -> Result<()> {
        println!("📋 Fetching projects from GitLab...");
        
        let url = format!("{}/api/v4/projects?simple=true&per_page=20", config.gitlab_url);
        
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
                for project in projects.iter() {
                    let name = project.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
                    let id = project.get("id").and_then(|i| i.as_i64()).unwrap_or(0);
                    let path_with_namespace = project.get("path_with_namespace")
                        .and_then(|p| p.as_str())
                        .unwrap_or("");
                    
                    println!("  • {} (ID: {}, Path: {})", name, id, path_with_namespace);
                }
                println!("\n💡 Use '/project <id_or_path>' to set a default project");
            }
        } else {
            println!("❌ Failed to fetch projects: {}", response.status());
        }
        
        Ok(())
    }
}