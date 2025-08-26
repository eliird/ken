use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub gitlab_url: String,
    pub api_token: String,
    pub default_project_id: Option<String>,
}

impl Config {
    pub fn new(gitlab_url: String, api_token: String) -> Self {
        Self {
            gitlab_url,
            api_token,
            default_project_id: None,
        }
    }

    pub fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Failed to get home directory")?;
        let config_dir = home.join(".ken");
        
        // Create directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }
        
        Ok(config_dir.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        
        if !path.exists() {
            anyhow::bail!("No configuration found. Please run 'ken auth login' first.");
        }
        
        let contents = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&contents)?;
        
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;
        
        Ok(())
    }

    pub fn prompt_for_login() -> Result<Self> {
        println!("GitLab Authentication Setup");
        println!("----------------------------");
        
        // Prompt for GitLab URL
        print!("Enter your GitLab URL (e.g., https://gitlab.com): ");
        io::stdout().flush()?;
        let mut gitlab_url = String::new();
        io::stdin().read_line(&mut gitlab_url)?;
        let mut gitlab_url = gitlab_url.trim().to_string();
        
        // Add https:// if no protocol is specified
        if !gitlab_url.starts_with("http://") && !gitlab_url.starts_with("https://") {
            gitlab_url = format!("https://{}", gitlab_url);
        }
        
        // Prompt for API token
        println!("\nTo create a personal access token:");
        println!("1. Go to {}/profile/personal_access_tokens", gitlab_url);
        println!("2. Create a token with 'api' scope");
        println!("3. Copy the token and paste it below");
        println!();
        
        print!("Enter your GitLab personal access token: ");
        io::stdout().flush()?;
        
        // Use rpassword to hide the token input
        let api_token = rpassword::read_password()?;
        
        // Optional: prompt for default project
        print!("\nEnter default project ID (optional, press Enter to skip): ");
        io::stdout().flush()?;
        let mut project_id = String::new();
        io::stdin().read_line(&mut project_id)?;
        let project_id = project_id.trim();
        
        let mut config = Config::new(gitlab_url, api_token);
        
        if !project_id.is_empty() {
            config.default_project_id = Some(project_id.to_string());
        }
        
        Ok(config)
    }

    pub async fn verify(&self) -> Result<()> {
        // Make a simple API call to verify the token works
        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/api/v4/user", self.gitlab_url))
            .header("PRIVATE-TOKEN", &self.api_token)
            .send()
            .await?;
        
        if response.status().is_success() {
            let user: serde_json::Value = response.json().await?;
            if let Some(username) = user.get("username").and_then(|u| u.as_str()) {
                println!("✓ Successfully authenticated as: {}", username);
            }
            Ok(())
        } else {
            anyhow::bail!("Authentication failed. Please check your token and URL.");
        }
    }
}