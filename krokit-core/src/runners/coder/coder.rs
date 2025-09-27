use std::sync::Arc;

use openai_dive::v1::resources::chat::ChatCompletionParametersBuilder;
use krokit_llm::{client::LlmClient, ChatMessage, ChatMessageContent};
use async_trait::async_trait;
use tracing::debug;

use crate::agent::brain::ThinkerDecision;
use crate::agent::{Agent, AgentBuilder, AgentError, Brain, ThinkerContext};
use crate::tools::types::{ContainsAnyTool, IntoToolBox};
use krokit_llm::tool::LlmToolCall;
use crate::tools::{AnyTool, BashTool, EditTool, FetchTool, FindTool, LsTool, MultiEditTool, ReadTool, TodoReadTool, TodoWriteTool, WriteTool, TodoStorage, FsOperationLog};

use super::prompt::{render_system_prompt_template, get_todo_read};

#[derive(Clone)]
pub struct CoderBrain {
    pub llm: Arc<LlmClient>,
    pub model: String,
    pub system_prompt_template: String,
    pub temperature: f32,
}

impl CoderBrain {
    pub fn new(llm: Arc<LlmClient>, model: String) -> Self {
        debug!(target: "brain::coder", provider =?llm.provider_name(), model = ?model);
        Self { 
            llm, 
            model,
            system_prompt_template: "{{CODER_BASE_PROMPT}}".to_string(),
            temperature: 0.3,
        }
    }

    pub fn with_custom_prompt(llm: Arc<LlmClient>, model: String, system_prompt_template: String, temperature: f32) -> Self {
        debug!(target: "brain::coder", provider =?llm.provider_name(), model = ?model);
        Self { 
            llm, 
            model,
            system_prompt_template,
            temperature,
        }
    }
}


#[async_trait]
impl Brain for CoderBrain {
    async fn next_step(&mut self, context: ThinkerContext) -> Result<ThinkerDecision, AgentError> {
        let mut trace = context.trace.read().await.clone();

        // Render the user's system prompt template
        let mut system_prompt = render_system_prompt_template(&self.system_prompt_template);
        
        // Add todo status if available
        if let Some(tool) = context.available_tools.get_tool("todo_read") {
            let todo_status = get_todo_read(&tool).await;
            system_prompt += &todo_status;
        }

        trace.insert(0, ChatMessage::System {
            content: ChatMessageContent::Text(system_prompt),
            name: None,
        });

        // get next step with custom temperature
        let request = ChatCompletionParametersBuilder::default()
            .model(&self.model)
            .messages(trace)
            .temperature(self.temperature)
            .build()
            .map_err(|e| AgentError::LlmError(e.to_string()))?;
        
        let brain_decision = self.llm.chat_with_tools(
                request,
                &context.available_tools.into_toolbox(),
                context.method)
                .await
                .map_err(|e| AgentError::LlmError(e.to_string()))?;
     
        // stop here if there's no other tool calls
        let message = brain_decision.choices.into_iter().next().unwrap().message;
        if let ChatMessage::Assistant { reasoning_content, content, tool_calls, .. } = &message {
            if tool_calls.as_ref().map_or(true, |calls| calls.is_empty()) {
                return Ok(ThinkerDecision::agent_pause(message));
            }
        } 
        Ok(ThinkerDecision::agent_continue(message))
    }
}


pub fn coder(llm: Arc<LlmClient>, model: String) -> impl Agent {
    // Create shared storage for todo tools
    let todo_storage = Arc::new(TodoStorage::new());
    
    // Create shared operation log for file system tools
    let fs_log = Arc::new(FsOperationLog::new());
    
    let bash = Box::new(BashTool::new());
    let edit = Box::new(EditTool::new(fs_log.clone()));
    let multiedit = Box::new(MultiEditTool::new(fs_log.clone()));
    let fetch = Box::new(FetchTool::new());
    let find = Box::new(FindTool::new());
    let ls = Box::new(LsTool::new());
    let read = Box::new(ReadTool::new(fs_log.clone()));
    let todoread = Box::new(TodoReadTool::new(todo_storage.clone()));
    let todowrite = Box::new(TodoWriteTool::new(todo_storage.clone()));
    let write = Box::new(WriteTool::new(fs_log.clone()));
    let toolbox: Vec<Box<dyn AnyTool>> = vec![bash, edit, multiedit, fetch, find, ls, read, todoread, todowrite, write];

    AgentBuilder::new(Box::new(CoderBrain::new(llm.clone(), model)))
    .tools(toolbox)
    .build()
}