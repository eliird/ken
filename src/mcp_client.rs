use std::time::Duration;
use anyhow::Result;
use mcp_core::{
    client::ClientBuilder,
    protocol::RequestOptions,
    transport::ClientSseTransportBuilder,
    types::ToolsListResponse,
};

pub struct MCPClient {
    pub inner: mcp_core::client::Client<mcp_core::transport::ClientSseTransport>,
}

impl MCPClient {
    pub async fn new(server_url: &str) -> Result<Self> {
        tracing::info!("Initializing MCP client with SSE transport at {}...", server_url);
        
        let transport = ClientSseTransportBuilder::new(server_url.to_string())
            .build();

        let client = ClientBuilder::new(transport)
            .set_protocol_version(mcp_core::types::ProtocolVersion::V2024_11_05)
            .set_client_info("ken_gitlab_client".to_string(), "0.1.0".to_string())
            .build();

        client.open().await?;
        client.initialize().await?;

        Ok(MCPClient { inner: client })
    }

    async fn _request(&self, endpoint: &str, params: Option<serde_json::Value>, options: RequestOptions) -> Result<serde_json::Value> {
        Ok(self.inner.request(endpoint, params, options).await?)
    }

    pub async fn get_tools_list(&self) -> Result<ToolsListResponse> {
        tracing::info!("Fetching available tools from MCP server...");
        let response = self._request("tools/list", None, RequestOptions::default().timeout(Duration::from_secs(10))).await?;
        
        let tools: ToolsListResponse = serde_json::from_value(response)?;
        tracing::info!("Found {} MCP tools", tools.tools.len());
        
        Ok(tools)
    }

    pub async fn run_tool(&self, tool_name: &str, tool_arguments: serde_json::Value) -> Result<serde_json::Value> {
        let jsonrpc_method = "tools/call";
        let jsonrpc_params = serde_json::json!({
            "name": tool_name,
            "arguments": tool_arguments
        });

        tracing::debug!("Calling MCP tool '{}' with args: {}", tool_name, tool_arguments);

        self._request(
            jsonrpc_method,
            Some(jsonrpc_params),
            RequestOptions::default().timeout(Duration::from_secs(30))
        ).await
    }
}