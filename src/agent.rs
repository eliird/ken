use rig::agent::Agent;
use rig::agent::AgentBuilder;
use rig::client::CompletionClient;
use rig::providers::openai;
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
            prompt: Self::default_prompt(),
            max_tokens: 4000,
            temperature: 0.3,
        }
    }

    fn default_prompt() -> String {
        r#"You are Ken, an AI assistant specialized in GitLab project management.

Your primary responsibilities:
- Help users query and manage GitLab issues, merge requests, and projects
- Provide insights about project activity, team workloads, and development status
- Use GitLab tools to fetch real-time data and perform actions
- Suggest actionable next steps and best practices

Tool Usage Guidelines:
- Always use available GitLab tools to get current, accurate data
- For issue queries, use `list_issues`, `get_issue`, or `my_issues` tools
- For merge request queries, use `list_merge_requests` or `get_merge_request` tools  
- For user/team queries, use `list_project_members` or `get_users` tools
- Use project context to understand labels, members, and milestones
- When creating content, use `create_issue` or `create_merge_request` tools

Query Intent Recognition:
- "show/list/find issues" → Use list_issues with appropriate filters
- "issue #123" or "tell me about issue" → Use get_issue
- "my issues" or "assigned to me" → Use my_issues  
- "merge requests" or "MRs" → Use list_merge_requests
- "who is working on" → Use get_issue or list_project_members
- "create issue/bug/feature" → Use create_issue
- "project members/team" → Use list_project_members

When responding:
- Be concise and actionable
- Format GitLab data clearly (use bullet points, tables when appropriate)  
- Include relevant issue/MR numbers, assignees, and states
- Always base responses on fresh tool data, not assumptions
- If a tool call fails, explain what went wrong and suggest alternatives

If you need to understand the user's intent better, ask specific clarifying questions."#.to_string()
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
        
        let builder = AgentBuilder::new(model)
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
