# KROKIT

krokit is a coding agent, your pair programming buddy that lives in the terminal. Written in rust with love <3

## About

KROKIT is a powerful AI-powered coding assistant designed to help developers with their daily programming tasks. It can read and edit files, run shell commands, search through codebases, and provide intelligent code suggestions - all from your terminal.

## Features

- **Interactive Terminal UI** - Beautiful TUI interface for chatting with your AI assistant
- **File Management** - Read, write, edit, and search files in your project
- **Shell Integration** - Execute commands and monitor terminal for errors
- **Multi-Provider Support** - Works with various AI providers (OpenRouter, OVHcloud, etc.)
- **Custom Agents** - Configure specialized agents with Model Context Protocol (MCP) servers
- **Headless Mode** - Script-friendly operation for automation
- **Shell Monitoring** - Automatic error detection and fix suggestions

## Installation

### Install latest stable release

Install the latest release with the following command:

```bash
curl -fsSL https://raw.githubusercontent.com/neur0map/krokit/main/install.sh | sh
```

The `krokit` binary will be installed in `$HOME/.local/bin`

### Build from source

```bash
git clone https://github.com/neur0map/krokit.git
cd krokit
cargo build --release
```

## Quick Start

### Configure a Provider

By default, krokit can use various AI providers. To configure your provider:

```bash
krokit auth
```

### Run Interactive Mode

Launch the interactive UI:

```bash
krokit
```

### Run in Headless Mode

Process prompts via stdin for scripting:

```bash
echo "Write a hello world in Python" | krokit
```

Get full conversation trace:

```bash
echo "Write a hello world in Python" | krokit --trace
```

Chain commands:

```bash
echo "Write a hello world" | krokit --trace | krokit "now run it!"
```

## Shell Integration

krokit can monitor your shell and provide automatic fixes when commands fail:

```bash
# Enable shell monitoring
krokit on

# Disable shell monitoring
krokit off

# Check status
krokit status
```

## Custom Agents

Create custom agents with specialized configurations and MCP servers. Place your configurations in `~/.config/krokit/agents/` and list available agents:

```bash
krokit agent list
```

Run a specific agent:

```bash
krokit <agent_name>
```

## Available Tools

- `bash` - Execute shell commands
- `edit` - Edit existing files
- `multiedit` - Make multiple edits to a file
- `write` - Create new files
- `read` - Read file contents
- `ls` - List directory contents
- `find` - Search for files
- `fetch` - Fetch web content
- `todoread`/`todowrite` - Manage task lists

## Development

Built with Rust, krokit consists of:
- `krokit-cli` - Main CLI application
- `krokit-core` - Core functionality and tools
- `krokit-llm` - LLM client implementations
- `krokit-macros` - Procedural macros

## License

Licensed under Apache 2.0 License. See LICENSE file for details.

## Author

Developed by neur0map

---

*krokit v0.1.0 - Your pair programming buddy in the terminal*