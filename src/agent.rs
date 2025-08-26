use rig::agent::Agent;
use rig::agent::AgentBuilder;
use rig::client::CompletionClient;
use rig::providers::openai;
use std::fs;
use std::io;

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

    
    pub fn with_gitlab_tools(gitlab_config: &crate::config::Config) -> Agent<openai::CompletionModel> {
        let config = AgentConfig::default();
        let model = openai::Client::from_url(&config.api_key, &config.base_url)
            .completion_model(&config.model_name);
        
        // Build the prompt with project context if available
        let mut enhanced_prompt = config.prompt.clone();
        if let Some(project_id) = &gitlab_config.default_project_id {
            if let Ok(context) = crate::context::ProjectContext::load(project_id) {
                if !context.labels.is_empty() || !context.users.is_empty() {
                    enhanced_prompt.push_str("\n\n");
                    enhanced_prompt.push_str(&context.to_prompt_context());
                }
            }
        }
        
        let gitlab_tool = crate::tools::ListIssuesTool::from_config(gitlab_config);
        let mut builder = AgentBuilder::new(model)
            .preamble(&enhanced_prompt)
            .temperature(config.temperature)
            .max_tokens(config.max_tokens)
            .tool(gitlab_tool);
        
        // Add context refresh tool if we have a project
        if let Some(context_tool) = crate::tools::RefreshContextTool::from_config(gitlab_config) {
            builder = builder.tool(context_tool);
        }
        
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
