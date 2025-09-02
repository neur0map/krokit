use crate::tools::McpClient;

use super::{StdioClient, HttpClient, SseClient};

#[derive(Debug, Clone)]
pub enum McpConfig {
    Stdio { command: String, args: Vec<String> },
    Http { url: String },
    Sse { url: String },
}

/// Factory function to create an MCP client from configuration
pub fn create_mcp_client(config: McpConfig) -> Box<dyn McpClient> {
    match config {
        McpConfig::Stdio { command, args } => {
            Box::new(StdioClient::new(command, args))
        }
        McpConfig::Http { url } => {
            Box::new(HttpClient::new(url))
        }
        McpConfig::Sse { url } => {
            Box::new(SseClient::new(url))
        }
    }
}