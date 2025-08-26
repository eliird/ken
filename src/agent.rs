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

pub struct KenAgent{
    pub agent: Agent<openai::CompletionModel>,
}

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
            max_tokens: 10000,
            temperature: 0.8,
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

    pub fn agent_with_config(cfg: AgentConfig) -> Agent<openai::CompletionModel>{
        Self::get_agent(&cfg)
    }

    fn get_agent(cfg: &AgentConfig) -> Agent<openai::CompletionModel>{
        let model = openai::Client::from_url(&cfg.api_key, &cfg.base_url).completion_model(&cfg.model_name);
        let builder = AgentBuilder::new(model)
        .preamble(&cfg.prompt)
        .temperature(cfg.temperature)
        .max_tokens(cfg.max_tokens);

        builder.build()
    }
}
