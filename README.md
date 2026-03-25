# GitAI

[![CI](https://github.com/nicholasgasior/gitai/actions/workflows/ci.yml/badge.svg)](https://github.com/nicholasgasior/gitai/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/gitai)](https://crates.io/crates/gitai)
[![License: MIT](https://img.shields.io/crates/l/gitai)](LICENSE.md)

AI-powered Git toolkit that enhances your workflow with intelligent commit messages, pull request descriptions, changelogs, release notes, and remote code synchronization. Integrates with multiple LLM providers to automate and improve your development process.

## Features

- **Smart Commit Messages** -- Generate meaningful commit messages based on staged changes using LLM analysis of your Git diff and context
- **Commit Message Completion** -- Complete partially typed commit messages with AI assistance using `--complete` and `--prefix` flags
- **Pull Request Descriptions** -- Generate comprehensive PR descriptions from branch comparisons, commit ranges, or single commits
- **Changelogs** -- Produce structured changelogs from commit history with optional `--save` to persist to `CHANGELOG.md`
- **Release Notes** -- Generate release notes between arbitrary Git references (tags, commits, branches)
- **Wire Operations** -- Synchronize code from remote repositories via shallow clone, sparse checkout, or partial clone strategies with `.gitwire.toml` configuration
- **Multiple LLM Providers** -- OpenAI, Anthropic (Claude), Google (Gemini), Groq, and OpenRouter out of the box
- **Git Config Integration** -- Layered configuration via environment variables, local git config, and global git config
- **Interactive TUI** -- Terminal UI for context selection with syntax-highlighted diffs, built with Ratatui

## Technology Stack

| Component           | Technology                               |
| ------------------- | ---------------------------------------- |
| Language            | Rust 2024 Edition                        |
| Async Runtime       | Tokio                                    |
| CLI Framework       | Clap 4 (derive)                          |
| Git Operations      | libgit2 (git2 crate)                     |
| LLM Integration     | `llm` crate (multi-provider abstraction) |
| Terminal UI         | Ratatui + Crossterm                      |
| Serialization       | Serde + TOML + JSON                      |
| HTTP                | Reqwest (native TLS)                     |
| Syntax Highlighting | Syntect                                  |

## Installation

### From source

```sh
git clone https://github.com/nicholasgasior/gitai.git
cd gitai
cargo build --release
```

The release build produces five binaries in `target/release/`:

| Binary              | Description                               |
| ------------------- | ----------------------------------------- |
| `git-message`       | Generate or complete commit messages      |
| `git-pr`            | Generate pull request descriptions        |
| `git-changelog`     | Generate changelogs                       |
| `git-release-notes` | Generate release notes                    |
| `git-wire`          | Synchronize code from remote repositories |

### Prerequisites

- Rust 1.85+ (edition 2024)
- A Git repository (commands must be run inside one)
- An API key for at least one supported LLM provider

## Configuration

Configuration uses a layered priority system:

1. **Environment variables** (highest priority)
2. **Local git config** (`.git/config`)
3. **Global git config** (`~/.gitconfig`)

### Set an API key

```sh
# Via environment variable
export GOOGLE_API_KEY="your-key"

# Via global git config
git config --global gitai.google-apikey "your-key"
```

### Set a model

```sh
git config gitai.openai-model "gpt-4o"
git config gitai.anthropic-model "claude-3-5-sonnet-latest"
```

### Set custom instructions

```sh
git config gitai.instructions "Use conventional commit format with scope"
```

### Supported providers and defaults

| Provider     | Default Model               |
| ------------ | --------------------------- |
| `google`     | gemini-3.0-flash            |
| `openai`     | gpt-4o                      |
| `anthropic`  | claude-3-5-sonnet-latest    |
| `groq`       | llama-3.3-70b-versatile     |
| `openrouter` | google/gemini-2.0-flash-001 |

## Usage

### Generate a commit message

```sh
# Generate and apply to staging area
git-message

# Complete a partial message
git-message --complete --prefix "feat: add user"

# Specify a provider and model
git-message --provider openai --model gpt-4o
```

### Generate a pull request description

```sh
# Compare current branch to main
git-pr --from main

# Compare specific commits
git-pr --from HEAD~3 --to HEAD

# Print to stdout
git-pr --from main --print
```

### Generate a changelog

```sh
# Between two references
git-changelog --from v0.1.3 --to HEAD

# Auto-detect from latest tag and save to CHANGELOG.md
git-changelog --save

# Specify output file
git-changelog --from v0.1.0 --save --file CHANGES.md
```

### Generate release notes

```sh
git-release-notes --from v0.1.3 --to v0.1.4
```

### Wire operations (code synchronization)

```sh
# Sync from a remote repository
git-wire sync --url https://github.com/org/repo --rev main --src lib --dst vendor/lib

# Check if synced code matches source
git-wire check --url https://github.com/org/repo --rev main --src lib --dst vendor/lib

# Save configuration to .gitwire.toml
git-wire sync --url https://github.com/org/repo --rev main --src lib --dst vendor/lib --save
```

## Development

### Build and run

```sh
cargo build
cargo run --bin git-message -- --print
```

### Run tests

```sh
cargo test
cargo test --features integration   # integration tests
```

### Lint and format

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

### Lint configuration

The project enforces strict linting via `Cargo.toml`:

- `clippy::all` at deny level
- `clippy::pedantic` at deny level with targeted allows
- `unsafe_code` is forbidden
- `unwrap_used` is denied -- all errors must be handled explicitly

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Write code following the project's lint rules (`cargo clippy -- -D warnings`)
4. Add or update tests for changed functionality
5. Ensure `cargo test` and `cargo fmt --check` pass
6. Open a pull request with a clear description of the change

## License

MIT License. See [LICENSE.md](LICENSE.md) for details.
