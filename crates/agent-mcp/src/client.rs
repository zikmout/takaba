use std::collections::HashMap;

use crate::config::{McpServerConfig, McpTransportConfig};
use crate::types::{McpTool, McpToolCall, McpToolResult};

pub struct McpClient {
    server_name: String,
    config: McpServerConfig,
    tools: Vec<McpTool>,
    connected: bool,
}

impl McpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            server_name: config.name.clone(),
            config,
            tools: Vec::new(),
            connected: false,
        }
    }

    /// Connect to the MCP server and discover tools.
    pub async fn connect(&mut self) -> anyhow::Result<()> {
        tracing::info!("connecting to MCP server: {}", self.server_name);

        match &self.config.transport {
            McpTransportConfig::Stdio { command, args, env } => {
                tracing::info!("stdio transport: {} {}", command, args.join(" "));
                // TODO: Use RMCP to establish stdio connection
                // For now, mark as connected without actual connection
                let _ = env;
            }
            McpTransportConfig::Sse { url } => {
                tracing::info!("SSE transport: {url}");
                // TODO: Use RMCP SSE transport
            }
            McpTransportConfig::Http { url } => {
                tracing::info!("HTTP transport: {url}");
                // TODO: Use RMCP HTTP transport
            }
        }

        self.connected = true;
        Ok(())
    }

    /// Discover available tools from the MCP server.
    pub async fn discover_tools(&mut self) -> anyhow::Result<Vec<McpTool>> {
        if !self.connected {
            anyhow::bail!("not connected to MCP server: {}", self.server_name);
        }

        // TODO: Implement actual tool discovery via RMCP
        // For now, return cached tools
        tracing::info!(
            "discovered {} tools from {}",
            self.tools.len(),
            self.server_name
        );

        Ok(self.tools.clone())
    }

    /// Call a tool on the MCP server.
    pub async fn call_tool(&self, call: &McpToolCall) -> anyhow::Result<McpToolResult> {
        if !self.connected {
            anyhow::bail!("not connected to MCP server: {}", self.server_name);
        }

        // Check permissions
        let is_write = is_write_operation(&call.tool);
        match self.config.permissions.check(is_write) {
            crate::permissions::PermissionResult::Allowed => {}
            crate::permissions::PermissionResult::Denied(reason) => {
                anyhow::bail!("MCP permission denied: {reason}");
            }
            crate::permissions::PermissionResult::NeedsConfirmation => {
                tracing::warn!(
                    "MCP tool {} requires confirmation (auto-allowing for now)",
                    call.tool
                );
            }
        }

        // TODO: Implement actual tool call via RMCP
        tracing::info!("calling MCP tool: {}.{}", call.server, call.tool);

        Ok(McpToolResult {
            content: vec![crate::types::McpContent::Text {
                text: "MCP tool call not yet implemented".into(),
            }],
            is_error: true,
        })
    }

    /// Shutdown the MCP server connection.
    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        if self.connected {
            tracing::info!("shutting down MCP server: {}", self.server_name);
            // TODO: Send shutdown via RMCP
            self.connected = false;
        }
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn server_name(&self) -> &str {
        &self.server_name
    }
}

/// Connect to all configured MCP servers and return clients.
pub async fn connect_all(
    configs: &[McpServerConfig],
) -> anyhow::Result<HashMap<String, McpClient>> {
    let mut clients = HashMap::new();

    for config in configs {
        if !config.enabled {
            tracing::info!("skipping disabled MCP server: {}", config.name);
            continue;
        }

        let mut client = McpClient::new(config.clone());
        if let Err(e) = client.connect().await {
            tracing::warn!("failed to connect to MCP server {}: {e}", config.name);
            continue;
        }
        clients.insert(config.name.clone(), client);
    }

    Ok(clients)
}

fn is_write_operation(tool_name: &str) -> bool {
    let write_patterns = [
        "write", "create", "update", "delete", "modify", "set", "push", "merge",
    ];
    let lower = tool_name.to_lowercase();
    write_patterns.iter().any(|p| lower.contains(p))
}
