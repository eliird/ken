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
    pub workload_data: WorkloadData,
    pub last_updated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct WorkloadData {
    pub user_assignments: HashMap<String, UserWorkload>,
    pub unassigned_issues: Vec<HotIssue>,
    pub total_open_issues: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UserWorkload {
    pub username: String,
    pub open_issues: Vec<HotIssue>,
    pub open_mrs: Vec<MergeRequest>,
    pub issue_count: usize,
    pub mr_count: usize,
    pub total_score: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MergeRequest {
    pub id: u32,
    pub title: String,
    pub source_branch: String,
    pub target_branch: String,
    pub state: String,
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
            workload_data: WorkloadData::default(),
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

    pub async fn fetch_from_gitlab(config: &crate::config::Config, project_id: &str) -> Result<Self> {
        let mut context = Self::new(project_id.to_string());
        
        let client = reqwest::Client::new();
        let base_url = &config.gitlab_url;
        let token = &config.api_token;
        
        // Fetch labels
        if let Ok(labels) = Self::fetch_labels(&client, base_url, token, project_id).await {
            context.labels = labels;
        }
        
        // Fetch project members
        if let Ok(users) = Self::fetch_project_members(&client, base_url, token, project_id).await {
            context.users = users;
        }
        
        // Fetch milestones
        if let Ok(milestones) = Self::fetch_milestones(&client, base_url, token, project_id).await {
            context.milestones = milestones;
        }
        
        // Fetch all open issues for activity tracking
        if let Ok(issues) = Self::fetch_all_open_issues(&client, base_url, token, project_id).await {
            context.hot_issues = issues;
        }
        
        // Fetch comprehensive workload data for each user
        context.workload_data = Self::fetch_workload_data(&client, base_url, token, project_id, &context.users).await?;
        
        context.update_timestamp();
        Ok(context)
    }
    
    async fn fetch_labels(client: &reqwest::Client, base_url: &str, token: &str, project_id: &str) -> Result<Vec<ProjectLabel>> {
        let url = format!("{}/api/v4/projects/{}/labels?per_page=100", base_url, urlencoding::encode(project_id));
        
        let response = client
            .get(&url)
            .header("PRIVATE-TOKEN", token)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Ok(Vec::new());
        }
        
        let labels: Vec<serde_json::Value> = response.json().await?;
        
        Ok(labels.into_iter().map(|label| {
            ProjectLabel {
                name: label.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                color: label.get("color").and_then(|c| c.as_str()).map(|s| s.to_string()),
                description: label.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
                usage_count: None,
            }
        }).collect())
    }
    
    async fn fetch_project_members(client: &reqwest::Client, base_url: &str, token: &str, project_id: &str) -> Result<Vec<ProjectUser>> {
        let url = format!("{}/api/v4/projects/{}/members/all?per_page=100", base_url, urlencoding::encode(project_id));
        
        let response = client
            .get(&url)
            .header("PRIVATE-TOKEN", token)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Ok(Vec::new());
        }
        
        let members: Vec<serde_json::Value> = response.json().await?;
        
        Ok(members.into_iter().map(|member| {
            ProjectUser {
                username: member.get("username").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                name: member.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()),
                email: member.get("email").and_then(|e| e.as_str()).map(|s| s.to_string()),
                role: member.get("access_level").and_then(|a| a.as_u64()).map(|level| {
                    match level {
                        10 => "Guest",
                        20 => "Reporter", 
                        30 => "Developer",
                        40 => "Maintainer",
                        50 => "Owner",
                        _ => "Member"
                    }.to_string()
                }),
            }
        }).collect())
    }
    
    async fn fetch_milestones(client: &reqwest::Client, base_url: &str, token: &str, project_id: &str) -> Result<Vec<ProjectMilestone>> {
        let url = format!("{}/api/v4/projects/{}/milestones?per_page=100", base_url, urlencoding::encode(project_id));
        
        let response = client
            .get(&url)
            .header("PRIVATE-TOKEN", token)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Ok(Vec::new());
        }
        
        let milestones: Vec<serde_json::Value> = response.json().await?;
        
        Ok(milestones.into_iter().map(|milestone| {
            ProjectMilestone {
                title: milestone.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                state: milestone.get("state").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                description: milestone.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
                due_date: milestone.get("due_date").and_then(|d| d.as_str()).map(|s| s.to_string()),
            }
        }).collect())
    }
    
    async fn fetch_all_open_issues(client: &reqwest::Client, base_url: &str, token: &str, project_id: &str) -> Result<Vec<HotIssue>> {
        let url = format!("{}/api/v4/projects/{}/issues?state=opened&per_page=100", base_url, urlencoding::encode(project_id));
        
        let response = client
            .get(&url)
            .header("PRIVATE-TOKEN", token)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Ok(Vec::new());
        }
        
        let issues: Vec<serde_json::Value> = response.json().await?;
        
        Ok(issues.into_iter().map(|issue| {
            let labels: Vec<String> = issue.get("labels")
                .and_then(|l| l.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            
            // Check if assignees array exists and get the first one
            let assignee = issue.get("assignees")
                .and_then(|a| a.as_array())
                .and_then(|arr| arr.first())
                .and_then(|a| a.get("username"))
                .and_then(|u| u.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    // Fallback to single assignee field
                    issue.get("assignee")
                        .and_then(|a| a.get("username"))
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string())
                });
                
            HotIssue {
                id: issue.get("iid").and_then(|i| i.as_u64()).unwrap_or(0) as u32,
                title: issue.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                assignee,
                labels,
                state: issue.get("state").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                updated_recently: true,
                priority: None,
            }
        }).collect())
    }

    async fn fetch_workload_data(
        client: &reqwest::Client, 
        base_url: &str, 
        token: &str, 
        project_id: &str,
        users: &[ProjectUser]
    ) -> Result<WorkloadData> {
        let mut workload_data = WorkloadData::default();
        
        println!("🔄 Fetching detailed workload data for {} users...", users.len());
        
        // Fetch issues and MRs for each user individually
        for (i, user) in users.iter().enumerate() {
            if i % 10 == 0 {
                println!("   Processing user {}/{}: {}", i + 1, users.len(), user.username);
            }
            
            let mut user_workload = UserWorkload {
                username: user.username.clone(),
                open_issues: Vec::new(),
                open_mrs: Vec::new(),
                issue_count: 0,
                mr_count: 0,
                total_score: 0,
            };
            
            // Fetch user's open issues
            if let Ok(issues) = Self::fetch_user_issues(client, base_url, token, project_id, &user.username).await {
                user_workload.issue_count = issues.len();
                user_workload.open_issues = issues;
            }
            
            // Fetch user's open MRs
            if let Ok(mrs) = Self::fetch_user_mrs(client, base_url, token, project_id, &user.username).await {
                user_workload.mr_count = mrs.len();
                user_workload.open_mrs = mrs;
            }
            
            // Calculate total score (issues + 2*MRs)
            user_workload.total_score = user_workload.issue_count + (user_workload.mr_count * 2);
            
            // Only store users with actual work
            if user_workload.issue_count > 0 || user_workload.mr_count > 0 {
                workload_data.user_assignments.insert(user.username.clone(), user_workload);
            }
        }
        
        // Fetch unassigned issues
        if let Ok(unassigned) = Self::fetch_unassigned_issues(client, base_url, token, project_id).await {
            workload_data.unassigned_issues = unassigned;
        }
        
        // Calculate total open issues
        let total_assigned: usize = workload_data.user_assignments.values().map(|w| w.issue_count).sum();
        workload_data.total_open_issues = total_assigned + workload_data.unassigned_issues.len();
        
        println!("✅ Workload data complete: {} active users, {} total issues", 
            workload_data.user_assignments.len(), workload_data.total_open_issues);
        
        Ok(workload_data)
    }
    
    async fn fetch_user_issues(
        client: &reqwest::Client, 
        base_url: &str, 
        token: &str, 
        project_id: &str,
        username: &str
    ) -> Result<Vec<HotIssue>> {
        let url = format!(
            "{}/api/v4/projects/{}/issues?assignee_username={}&state=opened&per_page=100",
            base_url, 
            urlencoding::encode(project_id),
            urlencoding::encode(username)
        );
        
        let response = client
            .get(&url)
            .header("PRIVATE-TOKEN", token)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Ok(Vec::new());
        }
        
        let issues: Vec<serde_json::Value> = response.json().await?;
        
        Ok(issues.into_iter().map(|issue| {
            let labels: Vec<String> = issue.get("labels")
                .and_then(|l| l.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
                
            HotIssue {
                id: issue.get("iid").and_then(|i| i.as_u64()).unwrap_or(0) as u32,
                title: issue.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                assignee: Some(username.to_string()),
                labels,
                state: issue.get("state").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                updated_recently: true,
                priority: None,
            }
        }).collect())
    }
    
    async fn fetch_user_mrs(
        client: &reqwest::Client, 
        base_url: &str, 
        token: &str, 
        project_id: &str,
        username: &str
    ) -> Result<Vec<MergeRequest>> {
        let url = format!(
            "{}/api/v4/projects/{}/merge_requests?assignee_username={}&state=opened&per_page=100",
            base_url, 
            urlencoding::encode(project_id),
            urlencoding::encode(username)
        );
        
        let response = client
            .get(&url)
            .header("PRIVATE-TOKEN", token)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Ok(Vec::new());
        }
        
        let mrs: Vec<serde_json::Value> = response.json().await?;
        
        Ok(mrs.into_iter().map(|mr| {
            MergeRequest {
                id: mr.get("iid").and_then(|i| i.as_u64()).unwrap_or(0) as u32,
                title: mr.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                source_branch: mr.get("source_branch").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                target_branch: mr.get("target_branch").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                state: mr.get("state").and_then(|s| s.as_str()).unwrap_or("").to_string(),
            }
        }).collect())
    }
    
    async fn fetch_unassigned_issues(
        client: &reqwest::Client, 
        base_url: &str, 
        token: &str, 
        project_id: &str
    ) -> Result<Vec<HotIssue>> {
        let url = format!(
            "{}/api/v4/projects/{}/issues?state=opened&per_page=100",
            base_url, 
            urlencoding::encode(project_id)
        );
        
        let response = client
            .get(&url)
            .header("PRIVATE-TOKEN", token)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Ok(Vec::new());
        }
        
        let issues: Vec<serde_json::Value> = response.json().await?;
        
        // Filter to only unassigned issues
        let unassigned: Vec<HotIssue> = issues.into_iter()
            .filter_map(|issue| {
                let has_assignee = issue.get("assignee").and_then(|a| a.as_object()).is_some() ||
                    issue.get("assignees").and_then(|a| a.as_array()).map_or(false, |arr| !arr.is_empty());
                
                if !has_assignee {
                    let labels: Vec<String> = issue.get("labels")
                        .and_then(|l| l.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();
                        
                    Some(HotIssue {
                        id: issue.get("iid").and_then(|i| i.as_u64()).unwrap_or(0) as u32,
                        title: issue.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
                        assignee: None,
                        labels,
                        state: issue.get("state").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                        updated_recently: true,
                        priority: None,
                    })
                } else {
                    None
                }
            })
            .collect();
        
        Ok(unassigned)
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