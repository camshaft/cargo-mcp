use crate::client::McpClient;
use reedline::{Span, Suggestion};
use rmcp::model::Tool;

/// Custom completer for MCP tools
pub struct Completer {
    commands: Vec<String>,
    built_in: Vec<String>,
    tools: Vec<(String, Tool)>,
}

impl Completer {
    pub fn new(client: &McpClient) -> Self {
        let built_in = vec![
            "h".to_string(),
            "help".to_string(),
            "list".to_string(),
            "tools".to_string(),
            "server".to_string(),
            "info".to_string(),
            "q".to_string(),
            "quit".to_string(),
            "exit".to_string(),
        ];

        let mut commands = built_in.clone();

        let tool_names = client.tool_names();
        let mut tools = vec![];
        for name in tool_names {
            commands.push(format!("tool:{name}"));
            commands.push(name.clone());

            let Some(tool) = client.get_tool(&name) else {
                continue;
            };

            tools.push((name, tool.clone()));
        }

        Self {
            commands,
            built_in,
            tools,
        }
    }

    pub fn commands(&self) -> &[String] {
        &self.commands
    }
}

impl reedline::Completer for Completer {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let mut completions = vec![];

        if line.contains(' ') {
            return completions;
        }

        // Complete command names
        let span = Span::new(0, pos);

        for command in &self.built_in {
            if command.starts_with(line) {
                completions.push(Suggestion {
                    value: command.clone(),
                    description: None,
                    extra: None,
                    span,
                    style: None,
                    append_whitespace: true,
                });
            }
        }

        let without_tool = line.trim_start_matches("tool:");
        for (name, tool) in &self.tools {
            if name.starts_with(without_tool) {
                completions.push(Suggestion {
                    value: name.clone(),
                    description: tool.description.as_ref().map(|v| v.to_string()),
                    extra: None,
                    span,
                    style: None,
                    append_whitespace: true,
                });
            }
        }

        completions
    }
}
