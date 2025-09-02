pub mod mcp;
pub mod mcp_stdio;
pub mod mcp_http;
pub mod mcp_sse;
pub mod mcp_config;

#[cfg(test)]
mod tests;

pub use mcp::{McpClient, McpToolDescription};
pub use mcp_config::{McpConfig, create_mcp_client};
pub use mcp_stdio::StdioClient;
pub use mcp_http::HttpClient;
pub use mcp_sse::SseClient;