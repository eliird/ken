use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ken")]
#[command(about = "AI-powered GitLab issue management assistant")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage authentication
    Auth {
        #[command(subcommand)]
        subcommand: AuthCommands,
    },
    
    /// Create or manage issues
    Issue {
        /// Natural language description of the issue
        description: String,
    },
    
    /// Summarize an existing issue
    Summarize {
        /// Issue ID or URL
        issue_id: String,
    },
    
    /// Suggest assignee for an issue
    Suggest {
        /// Issue ID or URL
        issue_id: String,
    },
    
    /// Check user workload
    Workload {
        /// Username (e.g., @alice)
        username: String,
    },
    
    /// Query issues using natural language
    Query {
        /// Natural language query (e.g., "What issues are assigned to irdali.durrani?")
        question: String,
        
        /// Optional: Specify project ID to query (overrides default)
        #[arg(short, long)]
        project: Option<String>,
    },
    
    /// Start interactive mode
    Interactive,
    
    /// Project management commands
    Project {
        #[command(subcommand)]
        subcommand: ProjectCommands,
    },
}

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Login to GitLab
    Login,
    
    /// Check authentication status
    Status,
    
    /// Logout (remove stored credentials)
    Logout,
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// List available projects
    List {
        /// Search for projects by name
        #[arg(short, long)]
        search: Option<String>,
        
        /// Show only your projects
        #[arg(short, long)]
        mine: bool,
    },
    
    /// Set default project
    Set {
        /// Project ID (can be namespace/project format)
        project_id: String,
    },
    
    /// Show current default project
    Current,
    
    /// Update project context (labels, users, team info)
    UpdateContext,
}