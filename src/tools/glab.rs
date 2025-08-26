use rig::tool::Tool;
use rig::completion::request::ToolDefinition;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::process::Command as AsyncCommand;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GlabCommandInput {
    /// The glab command to execute (e.g., "issue list --author=username")
    command: String,
    
    /// Optional: Additional arguments as a list
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Vec<String>>,
    
    /// Optional: Specify project (--repo flag)
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum GlabToolError {
    #[error("Command execution failed: {0}")]
    ExecutionError(String),
    #[error("Glab not found: {0}")]
    GlabNotFound(String),
    #[error("Invalid command: {0}")]
    InvalidCommand(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct GlabTool {
    default_project_id: Option<String>,
}

impl GlabTool {
    pub fn new(default_project_id: Option<String>) -> Self {
        Self {
            default_project_id,
        }
    }
    
    pub fn from_config(config: &crate::config::Config) -> Self {
        Self::new(config.default_project_id.clone())
    }
    
    fn build_command(&self, input: &GlabCommandInput) -> Vec<String> {
        let mut cmd_parts = vec!["glab".to_string()];
        
        // Parse the command string
        let command_parts: Vec<&str> = input.command.split_whitespace().collect();
        cmd_parts.extend(command_parts.iter().map(|s| s.to_string()));
        
        // Add additional args if provided
        if let Some(ref args) = input.args {
            cmd_parts.extend(args.clone());
        }
        
        // Add project flag if specified
        if let Some(ref project) = input.project {
            cmd_parts.push("--repo".to_string());
            cmd_parts.push(project.clone());
        } else if let Some(ref default_project) = self.default_project_id {
            cmd_parts.push("--repo".to_string());
            cmd_parts.push(default_project.clone());
        }
        
        // Force JSON output for structured parsing
        if !cmd_parts.iter().any(|arg| arg == "--json") {
            cmd_parts.push("--json".to_string());
        }
        
        cmd_parts
    }
}

impl Tool for GlabTool {
    const NAME: &'static str = "execute_glab_command";
    
    type Error = GlabToolError;
    type Args = GlabCommandInput;
    type Output = Value;
    
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: r#"Execute GitLab CLI (glab) commands to interact with GitLab. 
Common commands:
- "issue list" - List issues
- "issue list --author=username" - List issues by author
- "issue list --assignee=username" - List issues by assignee
- "issue list --state=opened" - List open issues
- "issue list --label=bug" - List issues by label
- "issue view 123" - View specific issue
- "mr list" - List merge requests
- "project list" - List projects

The tool automatically adds --json flag for structured output."#.to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The glab command to execute (e.g., 'issue list --author=username')"
                    },
                    "args": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Additional arguments as a list"
                    },
                    "project": {
                        "type": "string",
                        "description": "Specify project (--repo flag)"
                    }
                },
                "required": ["command"]
            }),
        }
    }
    
    async fn call(&self, input: Self::Args) -> Result<Self::Output, Self::Error> {
        // Build the complete command
        let cmd_parts = self.build_command(&input);
        
        if cmd_parts.is_empty() {
            return Err(GlabToolError::InvalidCommand("Empty command".to_string()));
        }
        
        // Execute the command
        let mut cmd = AsyncCommand::new(&cmd_parts[0]);
        if cmd_parts.len() > 1 {
            cmd.args(&cmd_parts[1..]);
        }
        
        let output = cmd.output().await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GlabToolError::GlabNotFound(
                    "glab command not found. Please install GitLab CLI: https://gitlab.com/gitlab-org/cli".to_string()
                )
            } else {
                GlabToolError::IoError(e)
            }
        })?;
        
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // Try to parse as JSON first
            if let Ok(json_value) = serde_json::from_str::<Value>(&stdout) {
                Ok(json!({
                    "success": true,
                    "command": cmd_parts.join(" "),
                    "data": json_value
                }))
            } else {
                // If not JSON, return as text
                Ok(json!({
                    "success": true,
                    "command": cmd_parts.join(" "),
                    "output": stdout.trim()
                }))
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(GlabToolError::ExecutionError(format!(
                "Command failed with exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            )))
        }
    }
}