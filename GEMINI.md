# GitAI Project Documentation

## Project Overview

GitAI is a Rust project that provides a suite of AI-powered tools for enhancing Git workflows and development processes. The project includes multiple binaries for specialized tasks:

2. **git-review** - Intelligent code reviews with quality dimensions
3. **git-pr** - Git pull request management
4. **git-changelog** - Dynamic changelog generation
5. **git-release-notes** - Release notes generation
6. **git-serve** - MCP server for AI tool integration
7. **git-presets** - Preset management
8. **git-wire** - A git subcommand that wires parts of other repositories' source code into the current repository in a declarative manner

The project also includes a sophisticated specification and planning system (`.specify`) that provides AI-assisted feature development workflows with templates and scripts to guide the development process.

## Project Structure

- `/src/` - Main source code with library and multiple binaries
- `/src/bin/` - Individual binary entry points (git-review, git-pr, git-changelog, etc.)
- `/commands/` - TOML configuration files that define AI command workflows
- `/.specify/` - Specification and planning system with templates and scripts
- `/target/` - Compiled output directory (git-ignored)

## Key Features

### gitai Tools
- **gitai** - Main CLI with AI-powered Git workflow assistance
- **git-review** - Intelligent code reviews with quality dimensions
- **git-pr** - Pull request management and analysis
- **git-changelog** - Dynamic changelog generation from commits
- **git-release-notes** - AI-generated release notes
- **git-serve** - MCP server for AI tool integration
- **git-presets** - Preset configurations for common workflows
- Multi-provider AI support (OpenAI, Anthropic, Google, Ollama, etc.)
- Interactive CLI for refining outputs
- Docker support for CI/CD integration

### Git-Wire (Repository Wiring Tool)
- Declarative cross-repository code synchronization
- JSON-based configuration for managing external code dependencies
- Multiple checkout methods (shallow, shallow_no_sparse, partial)
- Multi-threaded execution with single-threaded option
- Direct sync and check commands without configuration files

### Specification System
- AI-assisted feature specification generation
- Template-driven planning and task creation
- Automated workflow execution with defined phases
- Prerequisites checking and environment setup
- Task tracking and progress management

## Building and Running

### Prerequisites
- Rust and Cargo (latest stable version)
- Git 2.23.0 or newer

### Build Commands
```bash
# Build all binaries
cargo build

# Build in release mode
cargo build --release

# Build specific binary
cargo build --bin gitai
cargo build --bin git-review
cargo build --bin git-wire
```

### Running Commands
```bash
# Run individual tools (after building or installing)
git-review                  # Code review
git-pr                      # Pull request management
git-changelog               # Generate changelog
git-release-notes           # Generate release notes
git-serve                   # Start MCP server
git-wire sync               # Sync external repositories
git-wire check              # Check for differences

# Install binaries
cargo install --bin git-review
cargo install --bin git-wire
# etc. for other binaries
```

## Development Conventions

- The project follows Rust 2024 edition standards
- Code is licensed under MIT license
- Extensive dependency management with detailed configuration
- Comprehensive clippy linting rules for code quality
- The project uses TOML files for configuration and command definitions

## Project Commands

The `/commands/` directory contains AI workflow definitions in TOML format:
- `analyze.toml` - Analysis workflows
- `clarify.toml` - Clarification workflows
- `constitution.toml` - Constitution/structure workflows
- `implement.toml` - Implementation execution
- `plan.toml` - Planning workflows
- `specify.toml` - Specification generation
- `tasks.toml` - Task management

## Configuration

The `.specify/` directory contains:
- Template files for specifications, plans, and tasks
- Bash scripts for feature creation and environment setup
- Memory directory (likely for AI context persistence)

## Repository Status

The project is actively developed, providing a suite of AI-powered Git tools with:
- Multiple specialized binaries for different Git workflow tasks
- Comprehensive AI integration with multiple providers
- Specification-driven development system
- Active CI/CD with automated testing and releases

## Project Direction

gitai provides a comprehensive AI-assisted development environment that combines:
1. Git workflow automation (via multiple specialized tools)
2. Cross-repository code management (via git-wire)
3. Specification-driven development (via the .specify system)
4. AI command workflows (via the commands system)
