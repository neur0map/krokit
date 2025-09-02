use async_trait::async_trait;
use shai_llm::ToolDescription;

use crate::tools::{ToolResult, ToolCall};

#[derive(Debug, Clone)]
pub struct McpToolDescription {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
}

impl ToolDescription for McpToolDescription {
    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        Box::leak(self.description.clone().into_boxed_str())
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.parameters_schema.clone()
    }
}

#[async_trait]
pub trait McpClient: Send + Sync {
    async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn list_tools(&self) -> Result<Vec<McpToolDescription>, Box<dyn std::error::Error + Send + Sync>>;
    async fn execute_tool(&self, tool_call: ToolCall) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>>;
}