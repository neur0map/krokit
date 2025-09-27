use std::{collections::HashMap, io, time::Duration};
use krokit_llm::ToolCallMethod;

use crate::tui::App;

impl App<'_> {
    pub(crate) fn list_command() -> HashMap<(String, String),Vec<String>> {
        HashMap::from([
            (("/exit","exit from the tui"), vec![]),
            (("/auth","select a provider"), vec![]),
            (("/model","change model for current provider"), vec![]),
            (("/tc","set the tool call method: [fc | fc2 | so]"), vec!["method"]),
        ])
        .into_iter()
        .map(|((cmd,desc),args)|((cmd.to_string(),desc.to_string()),args.into_iter().map(|s|s.to_string()).collect()))
        .collect()
    }

    pub(crate) async fn handle_app_command(&mut self, command: &str) -> io::Result<()> {
        let mut parts = command.split_whitespace();
        let cmd = parts.next().unwrap();
        let args: Vec<&str> = parts.collect();

        match cmd {
            "/exit" => {
                self.exit = true;
            }
            "/model" => {
                // Open inline model selector
                let _ = self.open_model_selector().await;
            }
            "/tc" => {
                if let Some(ref agent) = self.agent {
                    match args.into_iter().next() {
                        Some("auto") => {
                            if let Ok(method) = agent.controller.set_method(Some(ToolCallMethod::Auto)).await {
                                self.input.alert_msg("llm will now try all method for tool calls", Duration::from_secs(3));
                                self.input.set_tool_call_method(method);
                            }
                        }
                        Some("fc") => {
                            if let Ok(method) = agent.controller.set_method(Some(ToolCallMethod::FunctionCall)).await {
                                self.input.alert_msg("llm will now use function calling api for tool calls", Duration::from_secs(3));
                                self.input.set_tool_call_method(method);
                            }
                        }
                        Some("fc2") => {
                            if let Ok(method) = agent.controller.set_method(Some(ToolCallMethod::FunctionCallRequired)).await {
                                self.input.alert_msg("llm will now use function calling in required mode for tool calls", Duration::from_secs(3));
                                self.input.set_tool_call_method(method);
                            }
                        }
                        Some("so") => {
                            if let Ok(method) = agent.controller.set_method(Some(ToolCallMethod::StructuredOutput)).await {
                                self.input.alert_msg("llm will now use structured output for tool calls", Duration::from_secs(3));
                                self.input.set_tool_call_method(method);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                self.input.alert_msg("command unknown", Duration::from_secs(1));
            }
        }
        Ok(())
    }
}
