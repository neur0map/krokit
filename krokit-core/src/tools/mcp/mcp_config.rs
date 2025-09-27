use crate::tools::McpClient;
use serde::{Serialize, Deserialize};

use super::{StdioClient, HttpClient, SseClient};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpConfig {
    #[serde(rename = "stdio")]
    Stdio { command: String, args: Vec<String> },
    #[serde(rename = "http")]
    Http { url: String, bearer_token: Option<String> },
    #[serde(rename = "sse")]
    Sse { url: String },
}

/// Factory function to create an MCP client from configuration
pub fn create_mcp_client(config: McpConfig) -> Box<dyn McpClient> {
    match config {
        McpConfig::Stdio { command, args } => {
            Box::new(StdioClient::new(command, args))
        }
        McpConfig::Http { url, bearer_token } => {
            Box::new(HttpClient::new_with_auth(url, bearer_token))
        }
        McpConfig::Sse { url } => {
            Box::new(SseClient::new(url))
        }
    }
}