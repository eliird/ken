use anyhow::Result;
use rustyline::{Editor, Helper};
use rustyline::completion::{Completer, Pair};
use rustyline::hint::Hinter;
use rustyline::highlight::Highlighter;
use rustyline::validate::Validator;
use rustyline::Context;
use crate::config::Config;
use crate::agent::KenAgent;
use crate::context::ProjectContext;
use crate::mcp_client::MCPClient;
use crate::gitlab_tools::GitLabTools;
use rig::agent::Agent;
use rig::providers::openai;
use rig::completion::Chat;
use mcp_core::types::ToolsListResponse;
use tokio::process::{Child, Command};
use std::time::Duration;

#[derive(Clone)]
pub struct KenCompleter {
    commands: Vec<String>,
}

impl KenCompleter {
    fn new() -> Self {
        Self {
            commands: vec![
                "/help".to_string(),
                "/login".to_string(),
                "/logout".to_string(),
                "/status".to_string(),
                "/projects".to_string(),
                "/project".to_string(),
                "/current".to_string(),
                "/context".to_string(),
                "/update-context".to_string(),
                "/list-tools".to_string(),
                "/restart-mcp".to_string(),
                "/issues".to_string(),
                "/mrs".to_string(),
                "/create".to_string(),
                "/workload".to_string(),
                "exit".to_string(),
                "quit".to_string(),
            ],
        }
    }
}

impl Completer for KenCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let start = line[..pos].rfind(' ').map(|i| i + 1).unwrap_or(0);
        let prefix = &line[start..pos];

        let matches: Vec<Pair> = self
            .commands
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|cmd| Pair {
                display: cmd.clone(),
                replacement: cmd.clone(),
            })
            .collect();

        Ok((start, matches))
    }
}

impl Hinter for KenCompleter {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<Self::Hint> {
        if pos < line.len() {
            return None;
        }

        let start = line.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let prefix = &line[start..];

        // Don't show hints if there's no input or just whitespace
        if prefix.is_empty() || prefix.trim().is_empty() {
            return None;
        }

        self.commands
            .iter()
            .find(|cmd| cmd.starts_with(prefix) && cmd.len() > prefix.len())
            .map(|cmd| cmd[prefix.len()..].to_string())
    }
}

impl Highlighter for KenCompleter {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> std::borrow::Cow<'b, str> {
        std::borrow::Cow::Borrowed(prompt)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        std::borrow::Cow::Borrowed(hint)
    }
}

impl Validator for KenCompleter {
    fn validate(
        &self,
        _ctx: &mut rustyline::validate::ValidationContext,
    ) -> rustyline::Result<rustyline::validate::ValidationResult> {
        Ok(rustyline::validate::ValidationResult::Valid(None))
    }
}

impl Helper for KenCompleter {}

pub struct KenSession {
    pub config: Option<Config>,
    pub editor: Editor<KenCompleter, rustyline::history::DefaultHistory>,
    pub agent: Option<Agent<openai::CompletionModel>>,
    pub mcp_client: Option<MCPClient>,
    pub mcp_tools: Option<ToolsListResponse>,
    pub mcp_server_process: Option<Child>,
}

impl KenSession {
    pub async fn new() -> Result<Self> {
        let mut editor = Editor::new().map_err(|e| anyhow::anyhow!("Failed to create editor: {}", e))?;
        
        // Set up autocomplete
        let completer = KenCompleter::new();
        editor.set_helper(Some(completer));
        
        // Try to load existing config, but don't fail if it doesn't exist
        let config = Config::load().ok();
        
        let agent = if config.is_some() {
            Some(KenAgent::default())
        } else {
            None
        };
        
        let mut session = KenSession {
            config,
            editor,
            agent,
            mcp_client: None,
            mcp_tools: None,
            mcp_server_process: None,
        };
        
        // Start MCP server immediately if we have config
        if session.config.is_some() {
            if let Err(e) = session.start_mcp_server().await {
                println!("⚠️  GitLab MCP server failed to start: {}", e);
                println!("    You can try restarting with /logout and /login");
            }
        }
        
        Ok(session)
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
                        self.cleanup().await;
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
                    self.cleanup().await;
                    break;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    // Ctrl-D
                    println!("👋 Goodbye!");
                    self.cleanup().await;
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
        println!("💡 Type '/help' for commands or 'exit' to quit.");
        println!("⌨️  Use TAB for autocompletion, UP/DOWN for history.\n");
    }
    
    async fn process_input(&mut self, input: &str) -> Result<()> {
        if input.starts_with('/') {
            self.handle_command(input).await
        } else {
            self.handle_query(input).await
        }
    }
    
    async fn handle_command(&mut self, command: &str) -> Result<()> {
        // Handle commands with parameters
        if command.starts_with("/issues") {
            return self.handle_issues_command(command).await;
        } else if command.starts_with("/mrs") {
            return self.handle_mrs_command(command).await;
        } else if command.starts_with("/project ") {
            return self.handle_project_command(command).await;
        }
        
        // Handle exact match commands
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
                println!("  /context        - View cached project context");
                println!("  /update-context - Update project context from GitLab");
                println!("  /list-tools     - List available GitLab MCP tools");
                println!("  /restart-mcp    - Restart GitLab MCP server");
                println!("  /issues [filter] - List project issues (optional: filter text)");
                println!("  /mrs [filter]    - List merge requests (optional: filter text)");
                println!("  /create         - Create new issue or merge request");
                println!("  /workload       - Show team workload distribution");
                println!("  exit            - Quit Ken");
            }
            "/login" => {
                println!("🔐 GitLab Authentication Setup");
                let new_config = Config::prompt_for_login()?;
                
                println!("🔄 Verifying credentials...");
                new_config.verify().await?;
                
                new_config.save()?;
                self.config = Some(new_config);
                
                // Start MCP server and initialize integration after successful login
                if let Err(e) = self.start_mcp_server().await {
                    println!("⚠️  GitLab MCP server failed to start: {}", e);
                }
                
                // Initialize agent with MCP tools if available
                if let (Some(config), Some(mcp_client), Some(tools)) = (&self.config, &self.mcp_client, &self.mcp_tools) {
                    self.agent = Some(KenAgent::with_mcp_tools(config, mcp_client, tools.clone()));
                } else {
                    self.agent = Some(KenAgent::default());
                }
                
                println!("✅ Login successful!");
            }
            "/logout" => {
                if self.config.is_some() {
                    let config_path = Config::config_path()?;
                    if config_path.exists() {
                        std::fs::remove_file(config_path)?;
                    }
                    self.config = None;
                    self.agent = None;
                    self.mcp_client = None;
                    self.mcp_tools = None;
                    
                    // Note: Keep MCP server running, just disconnect client
                    println!("🔌 Disconnected from GitLab MCP server");
                    
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
            "/context" => {
                if let Some(ref config) = self.config {
                    if let Some(ref project_id) = config.default_project_id {
                        match ProjectContext::load(project_id) {
                            Ok(context) => {
                                println!("📋 Context for project: {}", project_id);
                                println!("🕒 Last updated: {}", context.last_updated.as_deref().unwrap_or("Never"));
                                println!("🏷️  Labels: {}", context.labels.len());
                                println!("👥 Users: {}", context.users.len());
                                println!("🎯 Milestones: {}", context.milestones.len());
                                println!("🔥 Hot issues: {}", context.hot_issues.len());
                            }
                            Err(_) => {
                                println!("❌ No cached context found. Use '/update-context' first.");
                            }
                        }
                    } else {
                        println!("❌ No default project set.");
                    }
                } else {
                    println!("❌ Not authenticated. Use '/login' first.");
                }
            }
            "/update-context" => {
                if let Some(ref config) = self.config {
                    if let Some(ref project_id) = config.default_project_id {
                        println!("🔄 Updating project context from GitLab...");
                        match ProjectContext::fetch_from_gitlab(config, project_id).await {
                            Ok(context) => {
                                // Save the context to cache
                                if let Err(e) = context.save() {
                                    println!("⚠️  Context fetched but failed to save: {}", e);
                                } else {
                                    println!("✅ Project context updated and cached successfully!");
                                }
                                
                                // Reinitialize agent with updated context
                                if let (Some(config), Some(mcp_client), Some(tools)) = (&self.config, &self.mcp_client, &self.mcp_tools) {
                                    self.agent = Some(KenAgent::with_mcp_tools(config, mcp_client, tools.clone()));
                                } else {
                                    self.agent = Some(KenAgent::default());
                                }
                            }
                            Err(e) => {
                                println!("❌ Failed to update context: {}", e);
                            }
                        }
                    } else {
                        println!("❌ No default project set. Use '/project <id>' first.");
                    }
                } else {
                    println!("❌ Not authenticated. Use '/login' first.");
                }
            }
            "/list-tools" => {
                if let Some(ref tools) = self.mcp_tools {
                    println!("🔧 Available GitLab MCP Tools ({} total):", tools.tools.len());
                    println!("─────────────────────────────────────");
                    
                    for (i, tool) in tools.tools.iter().enumerate() {
                        let desc = tool.description.as_deref().unwrap_or("No description");
                        println!("{}. {} - {}", i + 1, tool.name, desc);
                    }
                    
                    println!("\n💡 These tools are available for natural language queries");
                } else {
                    println!("❌ No MCP tools available. Make sure you're logged in and MCP server is running.");
                }
            }
            "/restart-mcp" => {
                if self.config.is_some() {
                    println!("🔄 Restarting GitLab MCP server...");
                    
                    // Kill existing server
                    if let Some(mut process) = self.mcp_server_process.take() {
                        let _ = process.kill().await;
                        println!("🛑 Stopped existing MCP server");
                    }
                    
                    // Clear existing connections
                    self.mcp_client = None;
                    self.mcp_tools = None;
                    
                    // Restart server
                    match self.start_mcp_server().await {
                        Ok(_) => {
                            println!("✅ MCP server restarted successfully!");
                            
                            // Reinitialize agent with new MCP connection
                            if let (Some(config), Some(mcp_client), Some(tools)) = (&self.config, &self.mcp_client, &self.mcp_tools) {
                                self.agent = Some(KenAgent::with_mcp_tools(config, mcp_client, tools.clone()));
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to restart MCP server: {}", e);
                        }
                    }
                } else {
                    println!("❌ Not authenticated. Use '/login' first.");
                }
            }
            "/create" => {
                println!("✨ What would you like to create?");
                println!("1. Issue (bug report, feature request, task, etc.)");
                println!("2. Merge Request");
                println!();
                
                let readline = self.editor.readline("Enter choice (1 or 2): ");
                match readline {
                    Ok(choice) => {
                        match choice.trim() {
                            "1" => {
                                self.create_issue_with_template().await;
                            }
                            "2" => {
                                self.create_mr_with_template().await;
                            }
                            _ => {
                                println!("❌ Invalid choice. Please enter 1 or 2.");
                            }
                        }
                    }
                    Err(_) => println!("❌ Failed to read input."),
                }
            }
            "/workload" => {
                println!("📊 Analyzing team workload...");
                
                if let Some(ref config) = self.config {
                    match self.analyze_workload_direct(config).await {
                        Ok(()) => {
                            // Analysis completed and displayed
                        }
                        Err(e) => {
                            println!("❌ Failed to analyze workload: {}", e);
                        }
                    }
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
        if let Some(ref agent) = self.agent {
            if let Some(ref config) = self.config {
                if let Some(ref project_id) = config.default_project_id {
                    // Try to load context to enhance the query
                    let context_info = match ProjectContext::load(project_id) {
                        Ok(context) => context.to_prompt_context(),
                        Err(_) => "No project context available. Use '/update-context' to fetch it.".to_string()
                    };
                    
                    let enhanced_query = format!("Project Context:\n{}\n\nUser Query: {}", context_info, query);
                    
                    println!("🤖 Processing query...");
                    match agent.chat(&enhanced_query, vec![]).await {
                        Ok(response) => {
                            println!("\n📝 Response:\n{}", response);
                        }
                        Err(e) => {
                            println!("❌ Error processing query: {}", e);
                        }
                    }
                } else {
                    println!("❌ No project set. Use '/project <id>' to set a project first.");
                }
            } else {
                println!("❌ Not authenticated. Use '/login' first.");
            }
        } else {
            println!("❌ LLM agent not initialized. Use '/login' to initialize.");
        }
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
    
    async fn start_mcp_server(&mut self) -> Result<()> {
        let config = self.config.as_ref().ok_or_else(|| anyhow::anyhow!("No config available"))?;
        
        // If MCP server is already running, just try to reconnect
        if self.mcp_server_process.is_some() {
            println!("🔄 MCP server already running, reconnecting...");
            return self.connect_to_mcp_server().await;
        }
        
        // Start the GitLab MCP server as a subprocess
        println!("🚀 Starting GitLab MCP server...");
        
        let mut cmd = Command::new("node");
        cmd.current_dir("gitlab-mcp")
            .arg("build/index.js")
            .env("GITLAB_PERSONAL_ACCESS_TOKEN", &config.api_token)
            .env("GITLAB_API_URL", &config.gitlab_url)
            .env("SSE", "true")
            .kill_on_drop(true);
        
        // Set project ID if available
        if let Some(ref project_id) = config.default_project_id {
            cmd.env("GITLAB_PROJECT_ID", project_id);
        }
        
        let child = cmd.spawn().map_err(|e| anyhow::anyhow!("Failed to start MCP server: {}. Make sure Node.js is installed and gitlab-mcp is built.", e))?;
        self.mcp_server_process = Some(child);
        
        println!("⏳ Waiting for MCP server to start...");
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        self.connect_to_mcp_server().await
    }
    
    async fn connect_to_mcp_server(&mut self) -> Result<()> {
        // Connect to the MCP server
        let mcp_server_url = "http://localhost:3002/sse";
        println!("🔄 Connecting to GitLab MCP server at {}...", mcp_server_url);
        
        // Retry connection a few times as server might take time to start
        let mut retries = 5;
        while retries > 0 {
            match MCPClient::new(mcp_server_url).await {
                Ok(client) => {
                    println!("✅ Connected to MCP server");
                    
                    // Get available tools
                    match client.get_tools_list().await {
                        Ok(tools) => {
                            println!("📋 Loaded {} GitLab tools", tools.tools.len());
                            self.mcp_tools = Some(tools);
                            self.mcp_client = Some(client);
                            return Ok(());
                        }
                        Err(e) => {
                            println!("⚠️  Failed to get tools list: {}", e);
                            break;
                        }
                    }
                }
                Err(_) => {
                    retries -= 1;
                    if retries > 0 {
                        println!("⏳ Retrying connection... ({} attempts left)", retries);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
        
        Err(anyhow::anyhow!("Failed to connect to MCP server after multiple attempts"))
    }
    
    async fn query_with_context(&self, query: &str) -> Result<String> {
        if let Some(ref agent) = self.agent {
            if let Some(ref config) = self.config {
                if let Some(ref project_id) = config.default_project_id {
                    // Try to load context to enhance the query
                    let context_info = match ProjectContext::load(project_id) {
                        Ok(context) => context.to_prompt_context(),
                        Err(_) => "No project context available. Use '/update-context' to fetch it.".to_string()
                    };
                    
                    let enhanced_query = format!(
                        "Project Context:\n{}\n\nCurrent Project: {}\nGitLab API URL: {}\n\nUser Query: {}", 
                        context_info, project_id, config.gitlab_url, query
                    );
                    
                    match agent.chat(&enhanced_query, vec![]).await {
                        Ok(response) => Ok(response),
                        Err(e) => Err(anyhow::anyhow!("Error processing query: {}", e))
                    }
                } else {
                    Err(anyhow::anyhow!("No project set. Use '/project <id>' to set a project first."))
                }
            } else {
                Err(anyhow::anyhow!("Not authenticated. Use '/login' first."))
            }
        } else {
            Err(anyhow::anyhow!("LLM agent not initialized. Use '/login' to initialize."))
        }
    }

    async fn handle_issues_command(&mut self, command: &str) -> Result<()> {
        println!("📋 Fetching project issues...");
        
        let query = if command.len() > 8 && command.starts_with("/issues ") {
            let filter_text = &command[8..].trim();
            format!("List issues in this project that match or relate to: {}", filter_text)
        } else {
            "List the current open issues in this project".to_string()
        };
        
        match self.query_with_context(&query).await {
            Ok(response) => {
                println!("\n{}", response);
            }
            Err(e) => {
                println!("❌ {}", e);
            }
        }
        Ok(())
    }
    
    async fn handle_mrs_command(&mut self, command: &str) -> Result<()> {
        println!("🔀 Fetching merge requests...");
        
        let query = if command.len() > 5 && command.starts_with("/mrs ") {
            let filter_text = &command[5..].trim();
            format!("List merge requests in this project that match or relate to: {}", filter_text)
        } else {
            "List the current open merge requests in this project".to_string()
        };
        
        match self.query_with_context(&query).await {
            Ok(response) => {
                println!("\n{}", response);
            }
            Err(e) => {
                println!("❌ {}", e);
            }
        }
        Ok(())
    }
    
    async fn handle_project_command(&mut self, command: &str) -> Result<()> {
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
        Ok(())
    }
    
    fn get_issue_template() -> String {
        r#"## 背景

このissueが切られた経緯や背景情報を記入してください

## 作業項目

1. [ ] 実際に作業する内容を（可能であれば順番に）列挙してください

## 完了条件

* [ ] どのような状態になっていれば完了としてよいかの条件を列挙してください"#.to_string()
    }
    
    fn get_mr_template() -> String {
        r#"## 概要
（何を目的としたどんな変更か）

## 検証項目
（このMRの変更に対する検証の内容について）

## 重点レビュー箇所
（特にレビュワーに見てほしいものがあればリスト形式で記載。特になくてもいい）

## 関連Issue
tasks#"#.to_string()
    }
    
    async fn create_issue_with_template(&mut self) {
        println!("📝 Creating Issue with Template");
        println!("─────────────────────────────────");
        
        // Get issue title
        let title = match self.editor.readline("Issue Title: ") {
            Ok(t) if !t.trim().is_empty() => t.trim().to_string(),
            _ => {
                println!("❌ Issue title cannot be empty.");
                return;
            }
        };
        
        // Get background
        println!("\n💡 背景 (Background - why this issue is being created):");
        let background = match self.editor.readline("> ") {
            Ok(b) if !b.trim().is_empty() => b.trim().to_string(),
            _ => "詳細は後ほど記載".to_string()
        };
        
        // Get work items
        println!("\n📋 作業項目 (Work items - enter items separated by comma):");
        println!("   Example: APIの実装, テストの追加, ドキュメントの更新");
        let work_items_input = self.editor.readline("> ").unwrap_or_default();
        let work_items: Vec<String> = if !work_items_input.trim().is_empty() {
            work_items_input.split(',')
                .map(|s| format!("[ ] {}", s.trim()))
                .collect()
        } else {
            vec!["[ ] 作業項目を追加してください".to_string()]
        };
        
        // Get completion conditions
        println!("\n✅ 完了条件 (Completion conditions - enter conditions separated by comma):");
        println!("   Example: すべてのテストがパス, コードレビュー完了");
        let completion_input = self.editor.readline("> ").unwrap_or_default();
        let completion_conditions: Vec<String> = if !completion_input.trim().is_empty() {
            completion_input.split(',')
                .map(|s| format!("[ ] {}", s.trim()))
                .collect()
        } else {
            vec!["[ ] 完了条件を追加してください".to_string()]
        };
        
        // Build the issue description
        let mut description = format!("## 背景\n\n{}\n\n## 作業項目\n\n", background);
        for (i, item) in work_items.iter().enumerate() {
            description.push_str(&format!("{}. {}\n", i + 1, item));
        }
        description.push_str("\n## 完了条件\n\n");
        for condition in completion_conditions {
            description.push_str(&format!("* {}\n", condition));
        }
        
        // Create the issue
        println!("\n🔄 Creating issue with formatted template...");
        let query = format!(
            "Create a new GitLab issue with title: '{}' and description:\n{}",
            title, description
        );
        
        match self.query_with_context(&query).await {
            Ok(response) => {
                println!("\n✅ {}", response);
            }
            Err(e) => {
                println!("❌ {}", e);
            }
        }
    }
    
    async fn create_mr_with_template(&mut self) {
        println!("🔀 Creating Merge Request with Template");
        println!("─────────────────────────────────────────");
        
        // Get MR title
        let title = match self.editor.readline("MR Title: ") {
            Ok(t) if !t.trim().is_empty() => t.trim().to_string(),
            _ => {
                println!("❌ MR title cannot be empty.");
                return;
            }
        };
        
        // Get source branch
        let source_branch = match self.editor.readline("Source Branch: ") {
            Ok(b) if !b.trim().is_empty() => b.trim().to_string(),
            _ => {
                println!("❌ Source branch cannot be empty.");
                return;
            }
        };
        
        // Get target branch
        println!("Target Branch (default: main): ");
        let target_branch = match self.editor.readline("> ") {
            Ok(b) if !b.trim().is_empty() => b.trim().to_string(),
            _ => "main".to_string()
        };
        
        // Get overview
        println!("\n📄 概要 (Overview - what changes and why):");
        let overview = match self.editor.readline("> ") {
            Ok(o) if !o.trim().is_empty() => o.trim().to_string(),
            _ => "変更内容の概要".to_string()
        };
        
        // Get verification items
        println!("\n🔍 検証項目 (Verification items - how to test, separated by comma):");
        let verification_input = self.editor.readline("> ").unwrap_or_default();
        let verification_items = if !verification_input.trim().is_empty() {
            verification_input.split(',')
                .map(|s| format!("- {}", s.trim()))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            "- 検証項目を追加してください".to_string()
        };
        
        // Get review focus points
        println!("\n🎯 重点レビュー箇所 (Key review points - optional, separated by comma):");
        let review_input = self.editor.readline("> ").unwrap_or_default();
        let review_points = if !review_input.trim().is_empty() {
            review_input.split(',')
                .map(|s| format!("- {}", s.trim()))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            "特になし".to_string()
        };
        
        // Get related issue
        println!("\n🔗 関連Issue番号 (Related issue number, e.g., 1234):");
        let issue_number = self.editor.readline("> ").unwrap_or_default();
        let related_issue = if !issue_number.trim().is_empty() {
            format!("tasks#{}", issue_number.trim())
        } else {
            "tasks#".to_string()
        };
        
        // Build the MR description
        let description = format!(
            r#"## 概要
{}

## 検証項目
{}

## 重点レビュー箇所
{}

## 関連Issue
{}"#,
            overview, verification_items, review_points, related_issue
        );
        
        // Create the MR
        println!("\n🔄 Creating merge request with formatted template...");
        let query = format!(
            "Create a new GitLab merge request with title: '{}', source branch: '{}', target branch: '{}', and description:\n{}",
            title, source_branch, target_branch, description
        );
        
        match self.query_with_context(&query).await {
            Ok(response) => {
                println!("\n✅ {}", response);
            }
            Err(e) => {
                println!("❌ {}", e);
            }
        }
    }

    async fn analyze_workload_direct(&self, config: &Config) -> Result<()> {
        let gitlab = GitLabTools::new(config.clone());
        
        println!("🔄 Fetching project members...");
        let members = gitlab.get_project_members().await?;
        println!("👥 Found {} project members", members.len());
        
        println!("🔄 Analyzing individual workloads...");
        let mut workloads = Vec::new();
        
        for member in &members {
            let issues = gitlab.get_issues_by_assignee(&member.username).await.unwrap_or_default();
            let mrs = gitlab.get_mrs_by_assignee(&member.username).await.unwrap_or_default();
            let load_score = issues.len() + (mrs.len() * 2);
            
            if load_score > 0 {
                workloads.push((member, issues.len(), mrs.len(), load_score));
            }
        }
        
        // Sort by load score (highest first)
        workloads.sort_by(|a, b| b.3.cmp(&a.3));
        
        println!("\n📊 **Team Workload Analysis**\n");
        println!("| Full Name (username) | Role | Open Issues | Open MRs | Load Score | Status |");
        println!("|---------------------|------|------------|----------|------------|--------|");
        
        for (member, issues, mrs, load_score) in &workloads {
            let status = match *load_score {
                score if score > 8 => "🔴 High",
                score if score >= 4 => "🟡 Medium", 
                _ => "🟢 Low"
            };
            
            println!("| {} ({}) | {} | {} | {} | {} | {} |",
                member.name,
                member.username,
                member.role_name,
                issues,
                mrs,
                load_score,
                status
            );
        }
        
        // Get unassigned issues
        println!("\n🔄 Checking for unassigned work...");
        let all_issues = gitlab.get_all_open_issues().await?;
        let unassigned_issues: Vec<_> = all_issues.iter()
            .filter(|issue| issue.assignee.is_none())
            .collect();
        
        println!("\n📈 **Summary & Recommendations:**");
        println!("- 🔴 High workload (>8): {} members", workloads.iter().filter(|w| w.3 > 8).count());
        println!("- 🟡 Medium workload (4-8): {} members", workloads.iter().filter(|w| w.3 >= 4 && w.3 <= 8).count());
        println!("- 🟢 Low workload (<4): {} members", workloads.iter().filter(|w| w.3 < 4).count());
        println!("- 📋 Total active members: {}", workloads.len());
        println!("- ❓ Unassigned issues: {}", unassigned_issues.len());
        
        if !unassigned_issues.is_empty() {
            println!("\n🔗 **Unassigned Issues:**");
            for issue in unassigned_issues.iter().take(5) {
                println!("  - Issue #{}: {}", issue.iid, issue.title);
            }
            if unassigned_issues.len() > 5 {
                println!("  ... and {} more", unassigned_issues.len() - 5);
            }
        }
        
        Ok(())
    }

    async fn cleanup(&mut self) {
        if let Some(mut process) = self.mcp_server_process.take() {
            let _ = process.kill().await;
        }
    }
}