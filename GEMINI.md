# Ritex Project Documentation

## Project Overview

Ritex is a Rust workspace project that appears to be a development tool ecosystem focused on AI-powered Git workflow assistance and specification-driven development. The workspace contains two main crates:

1. **git-iris** - An AI-powered Git workflow assistant that enhances development processes with intelligent support for commit messages, code reviews, changelogs, and release notes
2. **git-wire** - A git subcommand that wires parts of other repositories' source code into the current repository in a declarative manner

The project also includes a sophisticated specification and planning system (`.specify`) that provides AI-assisted feature development workflows with templates and scripts to guide the development process.

## Workspace Structure

- `/crates/` - Contains the two main Rust crates (`git-iris` and `git-wire`)
- `/src/` - Main entry point (currently has a simple "Hello, world!" main.rs)
- `/commands/` - TOML configuration files that define AI command workflows
- `/.specify/` - Specification and planning system with templates and scripts
- `/target/` - Compiled output directory (git-ignored)

## Key Features

### Git-Iris (AI Git Assistant)
- AI-powered Git commit message generation
- Intelligent code reviews with 11 quality dimensions
- Dynamic changelog and release notes generation
- Multi-provider support (OpenAI, Anthropic, Google, Ollama, etc.)
- Interactive CLI for refining commits
- Docker support for CI/CD integration
- MCP (Model Context Protocol) server for AI tool integration

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
# Build all workspace crates
cargo build

# Build in release mode
cargo build --release

# Build specific crate
cargo build -p git-iris
cargo build -p git-wire
```

### Running Commands
```bash
# Run git-iris (if installed)
git-iris gen                    # Generate commit message
git-iris review                 # Code review
git-iris changelog            # Generate changelog
git-iris release-notes        # Generate release notes

# Run git-wire (if installed)
git wire sync                 # Sync external repositories
git wire check                # Check for differences

# Install crates
cargo install --path crates/git-iris
cargo install --path crates/git-wire
```

## Development Conventions

- The project follows Rust 2024 edition standards
- Code is licensed under various open-source licenses (Apache-2.0 for git-iris, MIT for git-wire)
- Both crates use extensive dependency management with detailed configuration
- The git-iris crate has comprehensive clippy linting rules for code quality
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

The project is still in early development, as indicated by:
- The main `src/main.rs` containing only a "Hello, world!" program
- The workspace structure suggesting this may be an experimental or work-in-progress setup
- The presence of both a specification system and multiple functional crates

## Potential Future Direction

Based on the components present, it appears Ritex might be evolving into a comprehensive AI-assisted development environment that combines:
1. Git workflow automation (via git-iris)
2. Cross-repository code management (via git-wire)
3. Specification-driven development (via the .specify system)
4. AI command workflows (via the commands system)