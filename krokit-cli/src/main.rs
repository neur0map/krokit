use headless::app::AppHeadless;
use clap::{Parser, Subcommand};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, EventStream},
    terminal::{disable_raw_mode, enable_raw_mode},
    ExecutableCommand,
};

use ringbuffer::RingBuffer;
use console::strip_ansi_codes;
use krokit_core::agent::LoggingConfig;
use krokit_core::config::config::KrokitConfig;
use krokit_core::config::agent::AgentConfig;
use krokit_core::agent::builder::AgentBuilder;
use krokit_core::runners::clifixer::fix::clifix;
use krokit_llm::{ChatMessage, ChatMessageContent};
use tui::auth::AppAuth;
use tui::theme::{apply_gradient, logo, logo_cyan, KROKIT_WHITE, KROKIT_YELLOW};
use tui::App;
use std::env;
use std::sync::Arc;
use std::io::{self, IsTerminal, Read, Write};
use std::process::Command;
use std::time::Duration;
use tokio::time::{sleep, interval};
use futures::StreamExt;

mod headless;
#[cfg(unix)]
mod fc;
#[cfg(unix)]
mod shell;

#[cfg(unix)]
use fc::history::CommandHistoryExt;
mod tui;

#[cfg(unix)]
use shell::pty::KrokitPtyManager;
#[cfg(unix)]
use shell::rc::{ShellType, get_shell};
#[cfg(unix)]
use fc::client::KrokitSessionClient;

use crate::headless::tools::list_all_tools;

#[derive(Parser)]
#[command(name = "krokit")]
#[command(about = "KROKIT - Coding agent, your pair programming buddy")]
#[command(subcommand_required = false)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Dump entire trace upon completion (headless mode only)
    #[arg(long, global = true)]
    trace: bool,
    /// the url to pull the default krokit config
    #[arg(long)]
    default_krokit_config_url: Option<String>,
    /// List all available tools
    #[arg(long)]
    list_tools: bool,
    /// Specify which tools to use (comma-separated)
    #[arg(long)]
    tools: Option<String>,
    /// Remove specific tools from the default set (comma-separated)
    #[arg(long)]
    remove: Option<String>,
    /// Show version information
    #[arg(short, long)]
    version: bool,
    /// Auto-fix mode: if no subcommand provided, these args go to fix
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Subcommand)]
enum AgentAction {
    /// List all available agents
    List,
    #[command(external_subcommand)]
    /// Run a specific agent by name
    Agent(Vec<String>),
}

#[derive(Subcommand)]
enum Commands {
    #[cfg(unix)]
    /// Start a PTY session with the specified shell
    On {
        /// Shell to run (defaults to $SHELL or /bin/sh)
        #[arg(short, long)]
        shell: Option<ShellType>,
        /// Suppress shell session restoration messages
        #[arg(long, default_value_t = true)]
        quiet: bool,
    },
    #[cfg(unix)]
    /// Exit the current PTY session
    Off,
    #[cfg(unix)]
    /// Is the session on or not
    Status,
    /// Configure KROKIT with your AI provider
    Auth,
    /// Agent management commands
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    #[cfg(unix)]
    /// Send pre-command hook (before command execution)
    #[command(hide = true)]
    Precmd {
        /// The command that is about to be executed
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    #[cfg(unix)]
    /// Send post-command hook (analyze last command)
    #[command(hide = true)]
    Postcmd {
        /// Exit code of the last command
        exit_code: i32,
        /// The command that was executed (optional)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    default_config(cli.default_krokit_config_url).await;

    match cli.command {
        #[cfg(unix)]
        Some(Commands::On { shell, quiet }) => {
            run_pty(shell, quiet)?;
        },
        #[cfg(unix)]
        Some(Commands::Off {  }) => {
            kill_pty()?;
        },
        #[cfg(unix)]
        Some(Commands::Status {  }) => {
            pty_status()?;
        },
        Some(Commands::Auth {  }) => {
            handle_config().await?;
        },
        Some(Commands::Agent { action }) => {
            handle_agent_command(action).await?;
        },
        #[cfg(unix)]
        Some(Commands::Precmd { command }) => {
            let command_str = command.join(" ");
            handle_precmd(command_str)?;
        },
        #[cfg(unix)]
        Some(Commands::Postcmd { exit_code, command }) => {
            let command_str = command.join(" ");
            handle_postcmd(exit_code, command_str).await?;
        },
        None => {
            // Check for stdin input or trailing arguments
            let stdin_input = if !io::stdin().is_terminal() {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)?;
                Some(buffer.trim().to_string()).filter(|s| !s.is_empty())
            } else {
                None
            };

            let mut messages = Vec::new();
            
            // Add stdin content as first message if present
            if let Some(stdin_content) = stdin_input {
                messages.push(stdin_content);
            }
            
            // Add arguments as second message if present
            if !cli.args.is_empty() {
                messages.push(cli.args.join(" "));
            }
            
            // Handle --list-tools flag
            if cli.list_tools {
                list_all_tools();
                return Ok(());
            }

            // Handle --version flag
            if cli.version {
                show_version()?;
                return Ok(());
            }

            if !messages.is_empty() || cli.list_tools {
                // Route to fix command with combined messages and global options
                handle_fix(messages, cli.tools, cli.remove, cli.trace, None).await?;
            } else {
                // No input, show TUI
                handle_main(None).await?;
            }
        }
    }

    Ok(())
}

async fn default_config(default_config_url: Option<String>) {
    if KrokitConfig::load().is_ok() {
        return;
    }

    let default_url = match default_config_url {
        Some(url) => url,
        None => "https://raw.githubusercontent.com/neur0map/krokit/refs/heads/main/.krokit.config".to_string()
    };

    let config = if let Ok(parsed_url) = default_url.parse() {
        KrokitConfig::pull_from_url(parsed_url).await.unwrap_or_else(|_| KrokitConfig::default())
    } else {
        KrokitConfig::default()
    };

    let _ = config.save();
}

async fn handle_main(agent_name: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let logo = logo();
    println!("{}", apply_gradient(&logo, KROKIT_YELLOW, KROKIT_YELLOW));
    let mut app = App::new();
    match app.run(agent_name).await {
        Err(e) => eprintln!("error: {}",e),
        _ => {}
    }
    Ok(())
}

async fn handle_config() -> Result<(), Box<dyn std::error::Error>> {
    let mut auth = AppAuth::new();
    auth.run().await;
    Ok(())
}

async fn ensure_config() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

async fn handle_fix(
    prompt: Vec<String>, 
    tools: Option<String>, 
    remove: Option<String>,
    trace: bool,
    agent_name: Option<String>
) -> Result<(), Box<dyn std::error::Error>> {
    let initial_trace: Vec<ChatMessage> = prompt.into_iter()
        .map(|p| ChatMessage::User { 
            content: ChatMessageContent::Text(p), 
            name: None 
        })
        .collect();
    
    AppHeadless::new().run(initial_trace, tools, remove, trace, agent_name).await
}

fn show_version() -> Result<(), Box<dyn std::error::Error>> {
    println!("{} version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    return Ok(());
}

#[cfg(unix)]
fn run_pty(shell: Option<ShellType>, quiet: bool) -> Result<(), Box<dyn std::error::Error>> {
    if env::var("KROKIT_SESSION_ID").is_ok() {
        eprintln!("Already inside a KROKIT session");
        return Ok(());
    }

    let mut pty = KrokitPtyManager::new()?;
    let shell = get_shell(shell)?;
    pty.start_session(shell, quiet)?;
    Ok(())
}


#[cfg(unix)]
fn kill_pty() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("KROKIT_SESSION_ID").is_err() {
        eprintln!("Not currently inside a KROKIT session");
        return Ok(());
    }

    let ppid = unsafe { libc::getppid() };
    unsafe {
        libc::kill(ppid, libc::SIGHUP);
    }
    std::process::exit(0);
}


#[cfg(unix)]
fn pty_status() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("KROKIT_SESSION_ID").is_ok() {
        eprintln!("krokit is enabled");
    } else {
        eprintln!("krokit is disabled");
    }
    Ok(())
}

#[cfg(unix)]
pub fn handle_precmd(command: String) -> Result<(), Box<dyn std::error::Error>> {
    env::var("KROKIT_SESSION_ID").ok()
        .and_then(|session_id| {
            let client = KrokitSessionClient::new(&session_id);
            client.session_exists().then(|| client.pre_command(&command))
        });
    Ok(())
}

#[cfg(unix)]
pub async fn handle_postcmd(exit_code: i32, command: String) -> Result<(), Box<dyn std::error::Error>> {
    env::var("KROKIT_SESSION_ID").ok()
        .and_then(|session_id| {
            let client = KrokitSessionClient::new(&session_id);
            client.session_exists().then(|| client.post_command( exit_code, &command))
        });

    match exit_code {
        0 => {
            // Success, do nothing
            return Ok(());
        },
        code if code >= 128 => {
            // Signal (Ctrl-C, SIGTERM, etc.), ignore
            return Ok(());
        },
        _ => {
            // Error occurred, analyze it
            let last_terminal_output = env::var("KROKIT_SESSION_ID").ok()
                .and_then(|session_id| {
                    let client = KrokitSessionClient::new(&session_id);
                    client.session_exists().then(|| client.get_last_commands(50).unwrap_or_else(|_| vec![].into()))
                });

            if let Some(cmd) = last_terminal_output {
                let trace = vec![ChatMessage::User { 
                    content: ChatMessageContent::Text(cmd.export_as_text()), 
                    name: None 
                }];
            
                let (llm, model) = KrokitConfig::get_llm().await?;
                
                enable_raw_mode().unwrap();
                let mut events = EventStream::new();
                let mut ticker = interval(Duration::from_millis(100));
                let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                let mut spinner_index = 0;
                
                let mut clifix_task = tokio::spawn(async move {
                    clifix(Arc::new(llm), model, trace).await
                });
                
                let result = loop {
                    tokio::select! {
                        result = &mut clifix_task => {
                            break result.unwrap();
                        }
                        
                        maybe_event = events.next() => {
                            if let Some(Ok(Event::Key(KeyEvent { code: KeyCode::Esc, .. }))) = maybe_event {
                                clifix_task.abort();
                                disable_raw_mode().unwrap();
                                eprintln!("\r\x1b[2K\x1b[2mCancelled.\x1b[0m");
                                return Ok(());
                            }
                        }
                        
                        _ = ticker.tick() => {
                            eprint!("\r\x1b[2mAnalyzing command... {} (Press ESC to cancel)\x1b[0m", spinner_chars[spinner_index]);
                            io::stdout().flush().unwrap();
                            spinner_index = (spinner_index + 1) % spinner_chars.len();
                        }
                    }
                };
                
                disable_raw_mode().unwrap();
                eprint!("\r\x1b[2K");
                
                match result {
                    Ok(res) => {
                        if let Some(rational) = &res.short_rational {
                            eprintln!("\n\x1b[2m{}\x1b[0m\n", rational);
                        }
                        eprint!("\x1b[38;5;206m❯\x1b[0m \x1b[1m{}\x1b[0m\n", &res.fixed_cli);
                        eprintln!("\n\x1b[2m ↵ Run • Esc / Ctrl+C Cancel\x1b[0m");
                        
                        io::stdout().execute(cursor::MoveUp(3)).unwrap();
                        io::stdout().execute(cursor::MoveToColumn((res.fixed_cli.len() + 3) as u16)).unwrap();
                        io::stdout().flush().unwrap();
                        enable_raw_mode().unwrap();
                        
                        loop {
                            if let Ok(Event::Key(KeyEvent { code, modifiers, .. })) = event::read() {
                                match (code, modifiers) {
                                    (KeyCode::Enter, _) => {
                                        disable_raw_mode().unwrap();
                                        io::stdout().execute(cursor::MoveDown(3)).unwrap();
                                        io::stdout().execute(cursor::MoveToColumn(0)).unwrap();
                                        println!();
                                        
                                        let mut cmd = Command::new("sh");
                                        cmd.arg("-c").arg(&res.fixed_cli);
                                        cmd.envs(env::vars());
                                        
                                        match cmd.status() {
                                            Ok(status) => {
                                                if status.success() {
                                                    shell::rc::write_to_shell_history(&res.fixed_cli);
                                                }
                                            }
                                            Err(e) => eprintln!("Failed to execute command: {}\n", e),
                                        }
                                        break;
                                    }
                                    (KeyCode::Esc, _) => {
                                        disable_raw_mode().unwrap();
                                        println!(); 
                                        break;
                                    }
                                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                                        disable_raw_mode().unwrap();
                                        println!();
                                        eprintln!("Exiting...");
                                        std::process::exit(0);
                                    }
                                    _ => {
                                        // Ignore other keys
                                    }
                                }
                            }
                        }
                    },
                    _ => {}
                }
            }  
        }
    }
    
    Ok(())
}

async fn handle_agent_command(action: AgentAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        AgentAction::List => {
            let agents = AgentConfig::list_agents()?;
            if agents.is_empty() {
                println!("No custom agents found.");
                println!("Create agent configs in ~/.config/krokit/agents/");
            } else {
                println!("Available agents:");
                
                // Find the longest agent name for alignment
                let max_name_len = agents.iter().map(|name| name.len()).max().unwrap_or(0);
                
                for agent in agents {
                    match AgentConfig::load(&agent) {
                        Ok(config) => {
                            println!("  \x1b[1m{:<width$}\x1b[0m \x1b[2m{}\x1b[0m", 
                                agent, 
                                config.description,
                                width = max_name_len
                            );
                        }
                        Err(_) => {
                            println!("  \x1b[1m{:<width$}\x1b[0m \x1b[2m(config error)\x1b[0m", 
                                agent,
                                width = max_name_len
                            );
                        }
                    }
                }
            }
        }
        AgentAction::Agent(args) => {
            if args.is_empty() {
                eprintln!("Error: Please specify an agent name");
                eprintln!("Usage: krokit agent <agent_name> [prompt]");
                return Ok(());
            }
            
            let agent_name = &args[0];
            let prompt_args: Vec<String> = args.iter().skip(1).cloned().collect();
            
            if prompt_args.is_empty() {
                // No prompt provided, start TUI mode with the agent
                handle_main(Some(agent_name.clone())).await?;
            } else {
                // Prompt provided, run in headless mode
                let prompt = prompt_args.join(" ");
                handle_fix(vec![prompt], None, None, false, Some(agent_name.clone())).await?;
            }
        }
    }
    Ok(())
}

