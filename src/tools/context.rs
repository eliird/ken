use rig::tool::Tool;
use rig::completion::request::ToolDefinition;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::context::{ProjectContext, ProjectLabel, ProjectUser, ProjectMilestone};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RefreshContextInput {
    /// Optional: Force refresh even if context is fresh
    #[serde(skip_serializing_if = "Option::is_none")]
    force_refresh: Option<bool>,
}

#[derive(Debug, thiserror::Error)]
pub enum ContextToolError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Context error: {0}")]
    ContextError(#[from] anyhow::Error),
}

#[derive(Debug, Clone)]
pub struct RefreshContextTool {
    gitlab_url: String,
    api_token: String,
    project_id: String,
}

impl RefreshContextTool {
    pub fn new(gitlab_url: String, api_token: String, project_id: String) -> Self {
        Self {
            gitlab_url,
            api_token,
            project_id,
        }
    }
    
    pub fn from_config(config: &crate::config::Config) -> Option<Self> {
        config.default_project_id.as_ref().map(|project_id| {
            Self::new(
                config.gitlab_url.clone(),
                config.api_token.clone(),
                project_id.clone(),
            )
        })
    }

    async fn fetch_labels(&self) -> Result<Vec<ProjectLabel>, ContextToolError> {
        let encoded_project_id = urlencoding::encode(&self.project_id);
        let url = format!("{}/api/v4/projects/{}/labels", self.gitlab_url, encoded_project_id);
        
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.api_token)
            .send()
            .await?;
        
        if response.status().is_success() {
            let labels: Vec<Value> = response.json().await?;
            let project_labels = labels.iter().map(|label| {
                ProjectLabel {
                    name: label.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                    color: label.get("color").and_then(|c| c.as_str()).map(|s| s.to_string()),
                    description: label.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
                    usage_count: label.get("open_issues_count").and_then(|c| c.as_u64()).map(|c| c as u32),
                }
            }).collect();
            
            Ok(project_labels)
        } else {
            Err(ContextToolError::ApiError(format!("Failed to fetch labels: {}", response.status())))
        }
    }

    async fn fetch_project_members(&self) -> Result<Vec<ProjectUser>, ContextToolError> {
        let encoded_project_id = urlencoding::encode(&self.project_id);
        let url = format!("{}/api/v4/projects/{}/members/all?per_page=100", self.gitlab_url, encoded_project_id);
        
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.api_token)
            .send()
            .await?;
        
        if response.status().is_success() {
            let members: Vec<Value> = response.json().await?;
            let project_users = members.iter().map(|member| {
                ProjectUser {
                    username: member.get("username").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                    name: member.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()),
                    email: member.get("email").and_then(|e| e.as_str()).map(|s| s.to_string()),
                    role: member.get("access_level").and_then(|r| r.as_u64()).map(|level| {
                        match level {
                            10 => "Guest",
                            20 => "Reporter", 
                            30 => "Developer",
                            40 => "Maintainer",
                            50 => "Owner",
                            _ => "Unknown",
                        }.to_string()
                    }),
                }
            }).collect();
            
            Ok(project_users)
        } else {
            Err(ContextToolError::ApiError(format!("Failed to fetch project members: {}", response.status())))
        }
    }

    async fn fetch_milestones(&self) -> Result<Vec<ProjectMilestone>, ContextToolError> {
        let encoded_project_id = urlencoding::encode(&self.project_id);
        let url = format!("{}/api/v4/projects/{}/milestones", self.gitlab_url, encoded_project_id);
        
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.api_token)
            .send()
            .await?;
        
        if response.status().is_success() {
            let milestones: Vec<Value> = response.json().await?;
            let project_milestones = milestones.iter().map(|milestone| {
                ProjectMilestone {
                    title: milestone.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                    state: milestone.get("state").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                    description: milestone.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
                    due_date: milestone.get("due_date").and_then(|d| d.as_str()).map(|s| s.to_string()),
                }
            }).collect();
            
            Ok(project_milestones)
        } else {
            Err(ContextToolError::ApiError(format!("Failed to fetch milestones: {}", response.status())))
        }
    }
}

impl Tool for RefreshContextTool {
    const NAME: &'static str = "refresh_project_context";
    
    type Error = ContextToolError;
    type Args = RefreshContextInput;
    type Output = Value;
    
    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Refresh project context by fetching current labels, users, and milestones from GitLab. Use this when you need up-to-date project information to make intelligent query decisions.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "force_refresh": {
                        "type": "boolean",
                        "description": "Force refresh even if context is fresh (default: false)"
                    }
                },
                "required": []
            }),
        }
    }
    
    async fn call(&self, input: Self::Args) -> Result<Self::Output, Self::Error> {
        // Load existing context
        let mut context = ProjectContext::load(&self.project_id)?;
        
        // Check if refresh is needed
        let force_refresh = input.force_refresh.unwrap_or(false);
        if !force_refresh && !context.is_stale() {
            return Ok(json!({
                "success": true,
                "message": "Context is fresh, no refresh needed",
                "labels_count": context.labels.len(),
                "users_count": context.users.len(),
                "milestones_count": context.milestones.len(),
                "last_updated": context.last_updated
            }));
        }
        
        // Fetch fresh data
        println!("🔄 Refreshing project context...");
        
        let (labels_result, users_result, milestones_result) = tokio::join!(
            self.fetch_labels(),
            self.fetch_project_members(),
            self.fetch_milestones()
        );
        
        // Update context with fresh data
        if let Ok(labels) = labels_result {
            context.labels = labels;
        }
        
        if let Ok(users) = users_result {
            context.users = users;
        }
        
        if let Ok(milestones) = milestones_result {
            context.milestones = milestones;
        }
        
        // TODO: Detect teams from user patterns or external config
        // For now, we could add some basic team detection logic
        
        context.update_timestamp();
        context.save()?;
        
        Ok(json!({
            "success": true,
            "message": "Project context refreshed successfully",
            "labels_count": context.labels.len(),
            "users_count": context.users.len(),
            "milestones_count": context.milestones.len(),
            "top_labels": context.labels.iter().take(10).map(|l| &l.name).collect::<Vec<_>>(),
            "sample_users": context.users.iter().take(5).map(|u| &u.username).collect::<Vec<_>>(),
            "last_updated": context.last_updated
        }))
    }
}