use rig::agent::Agent;
use rig::agent::AgentBuilder;
use rig::client::CompletionClient;
use rig::providers::openai;
use std::fs;
use std::io;
use mcp_core::types::ToolsListResponse;

pub struct AgentConfig{
    api_key: String,
    base_url: String,
    model_name: String,
    prompt: String,
    temperature: f64,
    max_tokens: u64,
}

pub struct KenAgent;

impl AgentConfig{
    pub fn default() -> Self{
        AgentConfig{
            model_name: String::from("Qwen/Qwen3-32B"),
            base_url: String::from("http://llm-api.fixstars.com/"),
            api_key: String::from(""),
            prompt: Self::read_prompt_file().unwrap_or_else(|err| {
                eprintln!("Failed to read prompt file: {}", err);
                String::new()
            }),
            max_tokens: 4000,
            temperature: 0.3,
        }
    }

    fn read_prompt_file() -> io::Result<String>{
        let prompt = "prompts/system_prompt.md";
        let prompt = fs::read_to_string(prompt);
        prompt
    }
}


impl KenAgent{
    pub fn default() -> Agent<openai::CompletionModel> {
        let config = AgentConfig::default();
        Self::get_agent(&config)
    }

    pub fn with_mcp_tools(
        gitlab_config: &crate::config::Config,
        mcp_client: &crate::mcp_client::MCPClient,
        tools: ToolsListResponse,
    ) -> Agent<openai::CompletionModel> {
        let config = AgentConfig::default();
        let model = openai::Client::from_url(&config.api_key, &config.base_url)
            .completion_model(&config.model_name);
        
        // Build the prompt with project context if available
        let mut enhanced_prompt = config.prompt.clone();
        if let Some(project_id) = &gitlab_config.default_project_id {
            enhanced_prompt.push_str(&format!("\n\n## Current GitLab Project\nProject: {}\n", project_id));
            
            // Add available MCP tools info
            enhanced_prompt.push_str("\n## Available GitLab Tools\n");
            for tool in &tools.tools {
                enhanced_prompt.push_str(&format!("- `{}`: {}\n", 
                    tool.name, 
                    tool.description.as_deref().unwrap_or("No description")
                ));
            }
        }
        
        let mut builder = AgentBuilder::new(model)
            .preamble(&enhanced_prompt)
            .temperature(config.temperature)
            .max_tokens(config.max_tokens);
        
        // Add all MCP tools dynamically
        let builder = tools.tools
            .into_iter()
            .fold(builder, |builder, tool| {
                builder.mcp_tool(tool, mcp_client.inner.clone().into())
            });

        builder.build()
    }

    fn get_agent(cfg: &AgentConfig) -> Agent<openai::CompletionModel>{
        let model = openai::Client::from_url(&cfg.api_key, &cfg.base_url)
            .completion_model(&cfg.model_name);
        AgentBuilder::new(model)
            .preamble(&cfg.prompt)
            .temperature(cfg.temperature)
            .max_tokens(cfg.max_tokens)
            .build()
    }
}
