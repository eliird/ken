use rig::tool::Tool;
use rig::completion::request::ToolDefinition;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListIssuesInput {
    /// Optional: Filter by assignee username (e.g., "irdali.durrani")
    #[serde(skip_serializing_if = "Option::is_none")]
    assignee_username: Option<String>,
    
    /// Optional: Filter by state (opened, closed, all)
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<String>,
    
    /// Optional: Filter by labels (comma-separated)
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<String>,
    
    /// Optional: Search within title and description
    #[serde(skip_serializing_if = "Option::is_none")]
    search: Option<String>,
    
    /// Optional: Project ID (uses default from config if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    project_id: Option<String>,
    
    /// Maximum number of issues to return (default: 20, max: 50)
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    
    /// Include issue descriptions (default: false to save context)
    #[serde(skip_serializing_if = "Option::is_none")]
    include_descriptions: Option<bool>,
}

#[derive(Debug, thiserror::Error)]
pub enum GitLabToolError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[derive(Debug, Clone)]
pub struct ListIssuesTool {
    gitlab_url: String,
    api_token: String,
    default_project_id: Option<String>,
}

impl ListIssuesTool {
    pub fn new(gitlab_url: String, api_token: String, default_project_id: Option<String>) -> Self {
        Self {
            gitlab_url,
            api_token,
            default_project_id,
        }
    }
    
    pub fn from_config(config: &crate::config::Config) -> Self {
        Self::new(
            config.gitlab_url.clone(),
            config.api_token.clone(),
            config.default_project_id.clone(),
        )
    }
}

impl Tool for ListIssuesTool {
    const NAME: &'static str = "list_gitlab_issues";
    
    type Error = GitLabToolError;
    type Args = ListIssuesInput;
    type Output = Value;
    
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List and search GitLab issues. Can filter by assignee, state, labels, and search terms. Server-side filtering for efficiency. Limited to 50 issues max.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "assignee_username": {
                        "type": "string",
                        "description": "Filter by assignee username (e.g., 'irdali.durrani')"
                    },
                    "state": {
                        "type": "string",
                        "enum": ["opened", "closed", "all"],
                        "description": "Filter by issue state"
                    },
                    "labels": {
                        "type": "string",
                        "description": "Filter by labels (comma-separated)"
                    },
                    "search": {
                        "type": "string",
                        "description": "Search within title and description"
                    },
                    "project_id": {
                        "type": "string",
                        "description": "Project ID (uses default from config if not provided)"
                    },
                    "limit": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 50,
                        "description": "Maximum number of issues to return (default: 20, max: 50)"
                    },
                    "include_descriptions": {
                        "type": "boolean",
                        "description": "Include issue descriptions (default: false to save context)"
                    }
                },
                "required": []
            }),
        }
    }
    
    async fn call(&self, input: Self::Args) -> Result<Self::Output, Self::Error> {
        // Determine project ID
        let project_id = input.project_id
            .or_else(|| self.default_project_id.clone())
            .ok_or_else(|| GitLabToolError::ConfigError("No project ID provided and no default set".to_string()))?;
        
        // Build the API URL with query parameters
        // URL encode the project ID to handle namespace/project format
        let encoded_project_id = urlencoding::encode(&project_id);
        let mut url = format!("{}/api/v4/projects/{}/issues", self.gitlab_url, encoded_project_id);
        let mut params = Vec::new();
        
        // Add filters
        if let Some(state) = input.state {
            params.push(format!("state={}", state));
        }
        
        if let Some(labels) = input.labels {
            params.push(format!("labels={}", labels));
        }
        
        if let Some(search) = input.search {
            params.push(format!("search={}", urlencoding::encode(&search)));
        }
        
        // Enforce reasonable limits (max 50 for server-side filtering, default 20)
        let limit = input.limit.unwrap_or(20).min(50);
        params.push(format!("per_page={}", limit));
        
        // Handle assignee filter
        if let Some(username) = input.assignee_username {
            // GitLab API supports assignee_username parameter directly
            params.push(format!("assignee_username={}", urlencoding::encode(&username)));
        }
        
        // Add parameters to URL
        if !params.is_empty() {
            url.push_str("?");
            url.push_str(&params.join("&"));
        }
        
        // Make the API request
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.api_token)
            .send()
            .await
            .map_err(|e| GitLabToolError::ApiError(format!("Failed to fetch issues: {}", e)))?;
        
        if response.status().is_success() {
            let issues: Vec<Value> = response.json().await
                .map_err(|e| GitLabToolError::ApiError(format!("Failed to parse issues: {}", e)))?;
            
            let include_descriptions = input.include_descriptions.unwrap_or(false);
            
            // Simplify the response to include only relevant fields
            let simplified_issues: Vec<Value> = issues.iter().map(|issue| {
                let mut simplified = json!({
                    "id": issue.get("iid"),
                    "title": issue.get("title"),
                    "state": issue.get("state"),
                    "assignee": issue.get("assignee").and_then(|a| a.get("username")),
                    "author": issue.get("author").and_then(|a| a.get("username")),
                    "created_at": issue.get("created_at"),
                    "updated_at": issue.get("updated_at"),
                    "labels": issue.get("labels"),
                    "web_url": issue.get("web_url"),
                });
                
                // Only include descriptions if explicitly requested
                if include_descriptions {
                    if let Some(desc) = issue.get("description").and_then(|d| d.as_str()) {
                        // Limit description to 100 characters to save context (handle UTF-8 properly)
                        let truncated = if desc.chars().count() > 100 {
                            let truncated_chars: String = desc.chars().take(100).collect();
                            format!("{}...", truncated_chars)
                        } else {
                            desc.to_string()
                        };
                        simplified["description"] = json!(truncated);
                    }
                }
                
                simplified
            }).collect();
            
            // Create a summary response
            let mut summary = json!({
                "success": true,
                "count": simplified_issues.len(),
                "total_fetched": issues.len(),
                "limit_applied": limit,
            });
            
            // Add summary statistics
            let open_count = simplified_issues.iter().filter(|i| 
                i.get("state").and_then(|s| s.as_str()) == Some("opened")
            ).count();
            let closed_count = simplified_issues.iter().filter(|i| 
                i.get("state").and_then(|s| s.as_str()) == Some("closed")
            ).count();
            
            summary["stats"] = json!({
                "open": open_count,
                "closed": closed_count,
            });
            
            // Add issues last to make the summary visible first
            summary["issues"] = json!(simplified_issues);
            
            // Add a note if we hit the limit
            if simplified_issues.len() == limit as usize {
                summary["note"] = json!("Result limit reached. There may be more issues. Use filters to narrow your search.");
            }
            
            Ok(summary)
        } else {
            Err(GitLabToolError::ApiError(format!("GitLab API error: {}", response.status())))
        }
    }
}