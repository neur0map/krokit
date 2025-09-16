use shai_llm::{ChatMessage, LlmClient};
use uuid::Uuid;
use std::sync::Arc;

use crate::tools::{AnyTool, BashTool, EditTool, FetchTool, FindTool, LsTool, MultiEditTool, ReadTool, TodoReadTool, TodoWriteTool, WriteTool, TodoStorage, FsOperationLog, create_mcp_client, get_mcp_tools};
use crate::config::agent::AgentConfig;
use crate::runners::coder::CoderBrain;
use super::Brain;
use super::AgentCore;
use super::claims::ClaimManager;
use super::AgentError;

/// Builder for AgentCore
pub struct AgentBuilder {
    pub session_id: String,
    pub brain: Box<dyn Brain>,
    pub goal: Option<String>,
    pub trace: Vec<ChatMessage>,
    pub available_tools: Vec<Box<dyn AnyTool>>,
    pub permissions: ClaimManager,
}

impl AgentBuilder {
    pub fn new(brain: Box<dyn Brain>) -> Self {
        Self {
            session_id: Uuid::new_v4().to_string(),
            brain: brain,
            goal: None,
            trace: vec![],
            available_tools: vec![],
            permissions: ClaimManager::new(),
        }
    }
}

impl AgentBuilder {
    pub fn id(mut self, session_id: &str) -> Self {
        self.session_id = session_id.to_string();
        self
    }
        
    pub fn brain(mut self, brain: Box<dyn Brain>) -> Self {
        self.brain = brain;
        self
    }
    
    pub fn goal(mut self, goal: &str) -> Self {
        self.goal = Some(goal.to_string());
        self
    }
    
    pub fn with_traces(mut self, trace: Vec<ChatMessage>) -> Self {
        self.trace = trace;
        self
    }

    pub fn tools(mut self, available_tools: Vec<Box<dyn AnyTool>>) -> Self {
        self.available_tools = available_tools;
        self
    }
    
    pub fn permissions(mut self, permissions: ClaimManager) -> Self {
        self.permissions = permissions;
        self
    }

    /// Enable sudo mode - bypasses all permission checks
    pub fn sudo(mut self) -> Self {
        self.permissions.sudo();
        self
    }

    /// Build the AgentCore with required runtime fields
    pub fn build(mut self) -> AgentCore {        
        if let Some(goal) = self.goal {
            self.trace.push(ChatMessage::User { content: shai_llm::ChatMessageContent::Text(goal.clone()), name: None });
        }
        
        AgentCore::new(
            self.session_id.clone(),
            self.brain,
            self.trace,
            self.available_tools,
            self.permissions
        )
    }

    /// Create an AgentBuilder from an AgentConfig
    pub async fn from_config(config: AgentConfig) -> Result<Self, AgentError> {
        // Create LLM client from provider config using the utility method
        let llm_client = Arc::new(
            LlmClient::create_provider(&config.llm_provider.provider, &config.llm_provider.env_vars)
                .map_err(|e| AgentError::LlmError(e.to_string()))?
        );
        
        // Create brain with custom system prompt and temperature
        let brain = Box::new(CoderBrain::with_custom_prompt(
            llm_client.clone(),
            config.llm_provider.model.clone(),
            config.system_prompt.clone(),
            config.temperature,
        ));

        // Create tools
        let tools = Self::create_tools_from_config(&config).await?;

        Ok(Self::new(brain)
            .tools(tools)
            .id(&format!("agent-{}", config.name)))
    }

    /// Create tools from config
    async fn create_tools_from_config(config: &AgentConfig) -> Result<Vec<Box<dyn AnyTool>>, AgentError> {
        let mut tools: Vec<Box<dyn AnyTool>> = Vec::new();

        // Create shared storage for todo tools
        let todo_storage = Arc::new(TodoStorage::new());
        
        // Create shared operation log for file system tools
        let fs_log = Arc::new(FsOperationLog::new());

        // Add builtin tools based on config
        let builtin_tools_to_add = if config.tools.builtin.contains(&"*".to_string()) {
            // Add all builtin tools
            vec!["bash", "edit", "multiedit", "fetch", "find", "ls", "read", "todo_read", "todo_write", "write"]
        } else {
            // Add only specified tools
            config.tools.builtin.iter().map(|s| s.as_str()).collect()
        };

        for tool_name in builtin_tools_to_add {
            match tool_name {
                "bash" => tools.push(Box::new(BashTool::new())),
                "edit" => tools.push(Box::new(EditTool::new(fs_log.clone()))),
                "multiedit" => tools.push(Box::new(MultiEditTool::new(fs_log.clone()))),
                "fetch" => tools.push(Box::new(FetchTool::new())),
                "find" => tools.push(Box::new(FindTool::new())),
                "ls" => tools.push(Box::new(LsTool::new())),
                "read" => tools.push(Box::new(ReadTool::new(fs_log.clone()))),
                "todo_read" => tools.push(Box::new(TodoReadTool::new(todo_storage.clone()))),
                "todo_write" => tools.push(Box::new(TodoWriteTool::new(todo_storage.clone()))),
                "write" => tools.push(Box::new(WriteTool::new(fs_log.clone()))),
                _ => return Err(AgentError::ConfigurationError(format!("Unknown builtin tool: {}", tool_name))),
            }
        }

        // Add MCP tools
        for (mcp_name, mcp_tool_config) in &config.tools.mcp {
            let mcp_client = create_mcp_client(mcp_tool_config.config.clone());
            
            // Get all tools from MCP client
            let all_mcp_tools = get_mcp_tools(mcp_client).await
                .map_err(|e| AgentError::ConfigurationError(format!("Failed to get tools from MCP client '{}': {}", mcp_name, e)))?;
            
            // Check if we should add all tools or filter by enabled_tools
            if mcp_tool_config.enabled_tools.contains(&"*".to_string()) {
                // Add all tools from this MCP client
                for tool in all_mcp_tools {
                    tools.push(tool);
                }
            } else {
                // Filter and add only enabled tools
                for tool in all_mcp_tools {
                    let tool_name = tool.name();
                    if mcp_tool_config.enabled_tools.contains(&tool_name) {
                        tools.push(tool);
                    }
                }
                
                // Check if all enabled tools were found (only when not using wildcard)
                for enabled_tool in &mcp_tool_config.enabled_tools {
                    let found = tools.iter().any(|t| t.name() == *enabled_tool);
                    if !found {
                        return Err(AgentError::ConfigurationError(format!("Tool '{}' not found in MCP client '{}'", enabled_tool, mcp_name)));
                    }
                }
            }
        }

        Ok(tools)
    }
}
