use crate::{client::McpClient, completer::Completer, prompt::ReplPrompt, term::*};
use nu_ansi_term::{Color, Style};
use reedline::{
    self, ColumnarMenu, DefaultHinter, DefaultValidator, Emacs, ExampleHighlighter,
    ExternalPrinter, FileBackedHistory, KeyCode, KeyModifiers, Keybindings, MenuBuilder, Reedline,
    ReedlineEvent, ReedlineMenu, Signal, default_emacs_keybindings,
};
use rmcp::model::Tool;
use std::{boxed::Box, ops::ControlFlow, path::PathBuf};

/// MCP REPL struct
pub struct Repl {
    prompt: ReplPrompt,
    history: Option<PathBuf>,
    history_capacity: Option<usize>,
    client: McpClient,
    keybindings: Keybindings,
    external_printer: ExternalPrinter<String>,
    hinter_style: Style,
}

impl Repl {
    /// Create a new MCP REPL with the given client
    pub fn new(client: McpClient) -> Self {
        let name = &client.server_info().server_info.name;
        let style = Style::new().italic().fg(Color::LightGray);
        let mut keybindings = default_emacs_keybindings();
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Tab,
            ReedlineEvent::Menu("completion_menu".to_string()),
        );
        let prompt = ReplPrompt::new(&paint_green_bold(&format!("{name}> ")));

        Self {
            history: None,
            history_capacity: None,
            hinter_style: style,
            prompt,
            client,
            keybindings,
            external_printer: ExternalPrinter::new(2048),
        }
    }

    /// Give your REPL a file based history saved at history_path
    #[allow(dead_code)] // TODO enable this
    pub fn with_history(mut self, history_path: PathBuf, capacity: usize) -> Self {
        self.history = Some(history_path);
        self.history_capacity = Some(capacity);
        self
    }

    async fn handle_command(
        &mut self,
        command: &str,
        args: Option<&str>,
    ) -> anyhow::Result<ControlFlow<()>> {
        match command {
            "h" | "help" => {
                if let Some(name) = args {
                    if let Some(tool) = self.client.get_tool(name) {
                        self.print_tool(tool);
                    }
                } else {
                    self.show_help();
                }
            }
            "list" | "tools" => {
                self.list_tools();
            }
            "server" => {
                self.show_server_info();
            }
            "q" | "quit" | "exit" => {
                return Ok(ControlFlow::Break(()));
            }
            _ => {
                let command = command.trim_start_matches("tool:");
                // Check if it's a tool name
                if self.client.tool_names().contains(&command.to_string()) {
                    let args_json = if let Some(args) = args {
                        Some(self.format_json_args(args)?)
                    } else {
                        None
                    };

                    let result = self.client.call_tool(command, args_json).await?;
                    println!("Result:");
                    println!("\n{}\n", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("Unknown command: {command}. Type 'help' for available commands.",);
                }
            }
        }
        Ok(ControlFlow::Continue(()))
    }

    fn show_help(&self) {
        println!("Available commands:");
        println!("  help          - Show this help message");
        println!("  list, tools   - List available tools");
        println!("  server, info  - Show server information");
        println!("  q, quit, exit    - Exit the REPL");
        println!();
        println!("Tool commands:");
        for tool_name in self.client.tool_names() {
            if let Some(tool) = self.client.get_tool(&tool_name) {
                println!(
                    "  {} - {}",
                    tool_name,
                    tool.description.as_deref().unwrap_or("No description")
                );
            }
        }
        println!();
        println!("To call a tool with arguments:");
        println!("  <tool_name> {{arg1: 'value1', arg2: 'value2'}}");
        println!();
        println!("To call a tool without arguments:");
        println!("  <tool_name>");
    }

    fn list_tools(&self) {
        println!("Available tools:");
        for tool_name in self.client.tool_names() {
            if let Some(tool) = self.client.get_tool(&tool_name) {
                self.print_tool(tool);
            }
        }
    }

    fn print_tool(&self, tool: &Tool) {
        println!("## {}\n", tool.name);

        if let Some(description) = tool.description.as_ref() {
            println!("{description}\n");
        }

        if let Ok(schema_str) = serde_json::to_string_pretty(&tool.input_schema) {
            println!("Schema:\n{schema_str}\n");
        }
    }

    fn show_server_info(&self) {
        let server_info = self.client.server_info();
        println!("Server Information:");
        println!("  Name: {}", server_info.server_info.name);
        println!("  Version: {}", server_info.server_info.version);
        println!("  Protocol: {}", server_info.protocol_version);

        if let Some(instructions) = &server_info.instructions {
            println!("  Instructions:");
            for line in instructions.lines() {
                println!("    {line}");
            }
        }
    }

    fn parse_line<'a>(&self, line: &'a str) -> Option<(&'a str, Option<&'a str>)> {
        let line = line.trim();

        if line.is_empty() {
            return None;
        }

        let Some((command, args)) = line.split_once(' ') else {
            return Some((line, None));
        };

        if args.is_empty() {
            Some((command, None))
        } else {
            Some((command, Some(args)))
        }
    }

    async fn process_line(&mut self, line: String) -> anyhow::Result<ControlFlow<()>> {
        if let Some((command, args)) = self.parse_line(&line) {
            self.handle_command(command, args).await
        } else {
            self.handle_command("help", None).await
        }
    }

    fn format_json_args(&self, args: &str) -> anyhow::Result<serde_json::Value> {
        // If that fails, try to parse as JSON5 for more flexible syntax
        match json5::from_str(args) {
            Ok(value) => Ok(value),
            Err(e) => Err(anyhow::anyhow!("Failed to parse JSON: {}", e)),
        }
    }

    fn build_line_editor(&mut self) -> anyhow::Result<Reedline> {
        let completer = Completer::new(&self.client);
        let valid_commands = completer.commands().to_vec();

        let completer = Box::new(completer);
        let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));
        let validator = Box::new(DefaultValidator);
        let mut line_editor = Reedline::create()
            .with_edit_mode(Box::new(Emacs::new(self.keybindings.clone())))
            .with_completer(completer)
            .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
            .with_highlighter(Box::new(ExampleHighlighter::new(valid_commands)))
            .with_validator(validator)
            .with_partial_completions(true)
            .with_quick_completions(true)
            .with_external_printer(self.external_printer.clone())
            .with_hinter(Box::new(
                DefaultHinter::default().with_style(self.hinter_style),
            ));

        if let Some(history_path) = &self.history {
            let capacity = self.history_capacity.unwrap();
            let history = FileBackedHistory::with_file(capacity, history_path.to_path_buf())?;
            line_editor = line_editor.with_history(Box::new(history));
        }

        Ok(line_editor)
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut line_editor = self.build_line_editor()?;

        loop {
            let sig = line_editor.read_line(&self.prompt)?;
            match sig {
                Signal::Success(line) => match self.process_line(line).await {
                    Ok(ControlFlow::Continue(())) => {}
                    Ok(ControlFlow::Break(())) => {
                        break;
                    }
                    Err(err) => {
                        println!("Error: {}", paint_yellow_bold(&err.to_string()));
                    }
                },
                Signal::CtrlC | Signal::CtrlD => {
                    break;
                }
            }
        }
        Ok(())
    }

    pub async fn run_non_interactive(&mut self) -> anyhow::Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};

        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            match self.process_line(line).await {
                Ok(ControlFlow::Continue(())) => {}
                Ok(ControlFlow::Break(())) => {
                    break;
                }
                Err(err) => {
                    println!("Error: {err}");
                }
            }
        }

        Ok(())
    }
}
