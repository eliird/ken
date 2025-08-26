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
- **ALWAYS use GitLab MCP tools for fresh data** - never rely only on cached context
- For issue queries, use `list_issues` with scope='all' for all issues, or specific assignee filters
- For merge request queries, use `list_merge_requests` with proper filtering options  
- For user/team queries, use `list_project_members` and `get_users` tools
- Use project context to understand labels, members, and milestones
- When creating content, use `create_issue` or `create_merge_request` tools
- **When asked to analyze workload, you MUST call the actual MCP tools, not use cached data**

Critical Tool Parameters:
- list_issues: ALWAYS use scope='all' to see all project issues, then filter by assignee
- list_merge_requests: Use assignee filters to get MRs per person
- list_project_members: Get full user details including names and roles

Query Intent Recognition:
- "show/list/find issues" → Use list_issues with appropriate filters
- "issue #123" or "tell me about issue" → Use get_issue
- "my issues" or "assigned to me" → Use my_issues  
- "merge requests" or "MRs" → Use list_merge_requests
- "who is working on" → Use get_issue or list_project_members
- "create issue/bug/feature" → Use create_issue
- "project members/team" → Use list_project_members
- "workload/analyze team" → Use list_project_members + list_issues (scope='all') + list_merge_requests
- "workload distribution" → 
  1. Get all members with list_project_members (get full names + roles)
  2. For each member, use list_issues with scope='all' and assignee=username filter  
  3. For each member, use list_merge_requests with assignee=username filter
  4. Count and calculate load scores

When responding:
- Be concise and actionable
- Format GitLab data clearly (use bullet points, tables when appropriate)  
- Include relevant issue/MR numbers, assignees, and states
- Always base responses on fresh tool data, not assumptions
- If a tool call fails, explain what went wrong and suggest alternatives

Workload Analysis Format:
- Create a table showing: Full Name (username) | Role | Open Issues | Open MRs | Load Score | Status
- Use FULL NAMES from list_project_members, not just usernames
- Calculate Load Score as: (Issues * 1) + (MRs * 2)  
- Status: 🔴 High (>8), 🟡 Medium (4-8), 🟢 Low (<4)
- Sort by Load Score (highest first)
- ONLY show members who actually have assigned work (Issues > 0 OR MRs > 0)
- Add summary with recommendations and unassigned work count

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
