use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProjectContext {
    pub project_id: String,
    pub labels: Vec<ProjectLabel>,
    pub users: Vec<ProjectUser>,
    pub milestones: Vec<ProjectMilestone>,
    pub teams: HashMap<String, Vec<String>>, // team name -> list of usernames
    pub hot_issues: Vec<HotIssue>,
    pub issue_patterns: IssuePatterns,
    pub last_updated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct IssuePatterns {
    pub most_used_labels: Vec<String>,
    pub active_assignees: Vec<String>,
    pub common_keywords: Vec<String>,
    pub priority_levels: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HotIssue {
    pub id: u32,
    pub title: String,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
    pub state: String,
    pub updated_recently: bool,
    pub priority: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectLabel {
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
    pub usage_count: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectUser {
    pub username: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectMilestone {
    pub title: String,
    pub state: String,
    pub description: Option<String>,
    pub due_date: Option<String>,
}

impl ProjectContext {
    pub fn new(project_id: String) -> Self {
        Self {
            project_id,
            labels: Vec::new(),
            users: Vec::new(),
            milestones: Vec::new(),
            teams: HashMap::new(),
            hot_issues: Vec::new(),
            issue_patterns: IssuePatterns::default(),
            last_updated: None,
        }
    }

    pub fn context_path(project_id: &str) -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Failed to get home directory"))?;
        let context_dir = home.join(".ken").join("contexts");
        
        // Create directory if it doesn't exist
        if !context_dir.exists() {
            fs::create_dir_all(&context_dir)?;
        }
        
        // Sanitize project ID for filename
        let safe_project_id = project_id.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
        Ok(context_dir.join(format!("{}.json", safe_project_id)))
    }

    pub fn load(project_id: &str) -> Result<Self> {
        let path = Self::context_path(project_id)?;
        
        if !path.exists() {
            return Ok(Self::new(project_id.to_string()));
        }
        
        let contents = fs::read_to_string(&path)?;
        let context: ProjectContext = serde_json::from_str(&contents)?;
        
        Ok(context)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::context_path(&self.project_id)?;
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        
        Ok(())
    }


    pub fn is_stale(&self) -> bool {
        // Consider context stale if older than 1 hour
        if let Some(last_updated) = &self.last_updated {
            if let Ok(updated_time) = chrono::DateTime::parse_from_rfc3339(last_updated) {
                let now = chrono::Utc::now();
                let duration = now.signed_duration_since(updated_time.with_timezone(&chrono::Utc));
                return duration.num_hours() > 1;
            }
        }
        true // No update time means definitely stale
    }

    pub fn update_timestamp(&mut self) {
        self.last_updated = Some(chrono::Utc::now().to_rfc3339());
    }
    
    /// Generate a context summary to include in LLM prompts
    pub fn to_prompt_context(&self) -> String {
        let mut context = format!("## Project Context for {}\n\n", self.project_id);
        
        // Available labels
        if !self.labels.is_empty() {
            context.push_str("**Available Labels:**\n");
            for label in self.labels.iter().take(20) {
                let usage = label.usage_count.map(|c| format!(" ({})", c)).unwrap_or_default();
                context.push_str(&format!("- `{}`: {}{}\n", 
                    label.name, 
                    label.description.as_deref().unwrap_or("No description"),
                    usage
                ));
            }
            context.push('\n');
        }
        
        // Project members
        if !self.users.is_empty() {
            context.push_str("**Project Members:**\n");
            for user in self.users.iter().take(15) {
                let role = user.role.as_deref().unwrap_or("Member");
                let name = user.name.as_deref().unwrap_or(&user.username);
                context.push_str(&format!("- `{}` ({}): {}\n", user.username, role, name));
            }
            context.push('\n');
        }
        
        // Teams
        if !self.teams.is_empty() {
            context.push_str("**Known Teams:**\n");
            for (team_name, members) in &self.teams {
                context.push_str(&format!("- `{}`: {}\n", team_name, members.join(", ")));
            }
            context.push('\n');
        }
        
        // Hot issues
        if !self.hot_issues.is_empty() {
            context.push_str("**Recent Activity:**\n");
            for issue in self.hot_issues.iter().take(10) {
                let assignee = issue.assignee.as_deref().unwrap_or("Unassigned");
                let labels = if issue.labels.is_empty() { 
                    "No labels".to_string() 
                } else { 
                    issue.labels.join(", ") 
                };
                context.push_str(&format!("- Issue #{}: {} (Assigned: {}, Labels: {})\n", 
                    issue.id, issue.title, assignee, labels));
            }
            context.push('\n');
        }
        
        // Issue patterns
        if !self.issue_patterns.most_used_labels.is_empty() {
            context.push_str("**Common Patterns:**\n");
            context.push_str(&format!("- Most used labels: {}\n", 
                self.issue_patterns.most_used_labels.join(", ")));
            context.push_str(&format!("- Active assignees: {}\n", 
                self.issue_patterns.active_assignees.join(", ")));
            if !self.issue_patterns.priority_levels.is_empty() {
                context.push_str(&format!("- Priority levels: {}\n", 
                    self.issue_patterns.priority_levels.join(", ")));
            }
            context.push('\n');
        }
        
        if let Some(last_updated) = &self.last_updated {
            context.push_str(&format!("*Context last updated: {}*\n", last_updated));
        }
        
        context
    }
}