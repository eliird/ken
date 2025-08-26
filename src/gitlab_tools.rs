use anyhow::Result;
use serde::{Deserialize, Serialize};
use crate::config::Config;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitLabUser {
    pub id: u64,
    pub username: String,
    pub name: String,
    pub email: Option<String>,
    pub state: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectMember {
    pub id: u64,
    pub username: String,
    pub name: String,
    pub email: Option<String>,
    pub state: String,
    pub avatar_url: Option<String>,
    pub access_level: u32,
    pub role_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitLabIssue {
    pub id: u64,
    pub iid: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
    pub assignee: Option<GitLabUser>,
    pub assignees: Vec<GitLabUser>,
    pub author: GitLabUser,
    pub labels: Vec<String>,
    pub milestone: Option<serde_json::Value>,
    pub web_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitLabMR {
    pub id: u64,
    pub iid: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
    pub assignee: Option<GitLabUser>,
    pub assignees: Vec<GitLabUser>,
    pub author: GitLabUser,
    pub source_branch: String,
    pub target_branch: String,
    pub web_url: String,
    pub merge_status: String,
}

pub struct GitLabTools {
    client: reqwest::Client,
    config: Config,
}

impl GitLabTools {
    pub fn new(config: Config) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    fn access_level_to_role(level: u32) -> String {
        match level {
            10 => "Guest".to_string(),
            20 => "Reporter".to_string(),
            30 => "Developer".to_string(),
            40 => "Maintainer".to_string(),
            50 => "Owner".to_string(),
            _ => format!("Level {}", level),
        }
    }

    pub async fn get_project_members(&self) -> Result<Vec<ProjectMember>> {
        let url = format!(
            "{}/api/v4/projects/{}/members/all?per_page=100",
            self.config.gitlab_url,
            urlencoding::encode(self.config.default_project_id.as_deref().unwrap_or(""))
        );

        let response = self.client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.config.api_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch project members: {}", response.status()));
        }

        let members: Vec<serde_json::Value> = response.json().await?;
        
        Ok(members.into_iter().map(|member| {
            let access_level = member.get("access_level").and_then(|a| a.as_u64()).unwrap_or(0) as u32;
            ProjectMember {
                id: member.get("id").and_then(|i| i.as_u64()).unwrap_or(0),
                username: member.get("username").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                name: member.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                email: member.get("email").and_then(|e| e.as_str()).map(|s| s.to_string()),
                state: member.get("state").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                avatar_url: member.get("avatar_url").and_then(|a| a.as_str()).map(|s| s.to_string()),
                access_level,
                role_name: Self::access_level_to_role(access_level),
            }
        }).collect())
    }

    pub async fn get_issues_by_assignee(&self, assignee: &str) -> Result<Vec<GitLabIssue>> {
        let url = format!(
            "{}/api/v4/projects/{}/issues?assignee_username={}&state=opened&per_page=100",
            self.config.gitlab_url,
            urlencoding::encode(self.config.default_project_id.as_deref().unwrap_or("")),
            urlencoding::encode(assignee)
        );

        let response = self.client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.config.api_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch issues for {}: {}", assignee, response.status()));
        }

        let issues: Vec<serde_json::Value> = response.json().await?;
        
        Ok(issues.into_iter().filter_map(|issue| {
            self.parse_issue(issue).ok()
        }).collect())
    }

    pub async fn get_mrs_by_assignee(&self, assignee: &str) -> Result<Vec<GitLabMR>> {
        let url = format!(
            "{}/api/v4/projects/{}/merge_requests?assignee_username={}&state=opened&per_page=100",
            self.config.gitlab_url,
            urlencoding::encode(self.config.default_project_id.as_deref().unwrap_or("")),
            urlencoding::encode(assignee)
        );

        let response = self.client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.config.api_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch MRs for {}: {}", assignee, response.status()));
        }

        let mrs: Vec<serde_json::Value> = response.json().await?;
        
        Ok(mrs.into_iter().filter_map(|mr| {
            self.parse_mr(mr).ok()
        }).collect())
    }

    pub async fn get_all_open_issues(&self) -> Result<Vec<GitLabIssue>> {
        let url = format!(
            "{}/api/v4/projects/{}/issues?state=opened&per_page=100",
            self.config.gitlab_url,
            urlencoding::encode(self.config.default_project_id.as_deref().unwrap_or(""))
        );

        let response = self.client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.config.api_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch all issues: {}", response.status()));
        }

        let issues: Vec<serde_json::Value> = response.json().await?;
        
        Ok(issues.into_iter().filter_map(|issue| {
            self.parse_issue(issue).ok()
        }).collect())
    }

    pub async fn get_project_labels(&self) -> Result<Vec<String>> {
        let url = format!(
            "{}/api/v4/projects/{}/labels?per_page=100",
            self.config.gitlab_url,
            urlencoding::encode(self.config.default_project_id.as_deref().unwrap_or(""))
        );

        let response = self.client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.config.api_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to fetch project labels: {}", response.status()));
        }

        let labels: Vec<serde_json::Value> = response.json().await?;
        
        Ok(labels.into_iter()
            .filter_map(|label| label.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
            .collect())
    }

    fn parse_user(&self, user_data: Option<&serde_json::Value>) -> Option<GitLabUser> {
        user_data.map(|user| {
            GitLabUser {
                id: user.get("id").and_then(|i| i.as_u64()).unwrap_or(0),
                username: user.get("username").and_then(|u| u.as_str()).unwrap_or("").to_string(),
                name: user.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                email: user.get("email").and_then(|e| e.as_str()).map(|s| s.to_string()),
                state: user.get("state").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                avatar_url: user.get("avatar_url").and_then(|a| a.as_str()).map(|s| s.to_string()),
            }
        })
    }

    fn parse_issue(&self, issue: serde_json::Value) -> Result<GitLabIssue> {
        Ok(GitLabIssue {
            id: issue.get("id").and_then(|i| i.as_u64()).unwrap_or(0),
            iid: issue.get("iid").and_then(|i| i.as_u64()).unwrap_or(0),
            title: issue.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
            description: issue.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
            state: issue.get("state").and_then(|s| s.as_str()).unwrap_or("").to_string(),
            created_at: issue.get("created_at").and_then(|c| c.as_str()).unwrap_or("").to_string(),
            updated_at: issue.get("updated_at").and_then(|u| u.as_str()).unwrap_or("").to_string(),
            assignee: self.parse_user(issue.get("assignee")),
            assignees: issue.get("assignees")
                .and_then(|a| a.as_array())
                .map(|arr| arr.iter().filter_map(|u| self.parse_user(Some(u))).collect())
                .unwrap_or_default(),
            author: self.parse_user(issue.get("author")).unwrap_or_default(),
            labels: issue.get("labels")
                .and_then(|l| l.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default(),
            milestone: issue.get("milestone").cloned(),
            web_url: issue.get("web_url").and_then(|w| w.as_str()).unwrap_or("").to_string(),
        })
    }

    fn parse_mr(&self, mr: serde_json::Value) -> Result<GitLabMR> {
        Ok(GitLabMR {
            id: mr.get("id").and_then(|i| i.as_u64()).unwrap_or(0),
            iid: mr.get("iid").and_then(|i| i.as_u64()).unwrap_or(0),
            title: mr.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
            description: mr.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
            state: mr.get("state").and_then(|s| s.as_str()).unwrap_or("").to_string(),
            created_at: mr.get("created_at").and_then(|c| c.as_str()).unwrap_or("").to_string(),
            updated_at: mr.get("updated_at").and_then(|u| u.as_str()).unwrap_or("").to_string(),
            assignee: self.parse_user(mr.get("assignee")),
            assignees: mr.get("assignees")
                .and_then(|a| a.as_array())
                .map(|arr| arr.iter().filter_map(|u| self.parse_user(Some(u))).collect())
                .unwrap_or_default(),
            author: self.parse_user(mr.get("author")).unwrap_or_default(),
            source_branch: mr.get("source_branch").and_then(|s| s.as_str()).unwrap_or("").to_string(),
            target_branch: mr.get("target_branch").and_then(|t| t.as_str()).unwrap_or("").to_string(),
            web_url: mr.get("web_url").and_then(|w| w.as_str()).unwrap_or("").to_string(),
            merge_status: mr.get("merge_status").and_then(|m| m.as_str()).unwrap_or("").to_string(),
        })
    }
}

impl Default for GitLabUser {
    fn default() -> Self {
        Self {
            id: 0,
            username: String::new(),
            name: String::new(),
            email: None,
            state: String::new(),
            avatar_url: None,
        }
    }
}