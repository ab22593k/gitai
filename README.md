# GitV

GitV is an AI-powered Git toolkit that enhances your Git workflow with intelligent commit messages, pull request generation, code reviews, changelogs, and more. It integrates with various LLM providers to automate and improve your development process.

## Features

- **Smart Commit Messages**: Generate meaningful commit messages based on your code changes
- **Pull Request Generation**: Automatically create detailed PR descriptions with context
- **Code Reviews**: Get AI-assisted code reviews with suggestions
- **Changelogs**: Generate release notes and changelogs from commit history
- **Multiple LLM Support**: Works with OpenAI, Anthropic, Google, and other providers
- **Git Config Integration**: Store configurations in Git config for project-specific settings
- **Wire Protocol Support**: Efficient caching and synchronization for remote repositories

## Installation

### From Source

```bash
git clone https://github.com/your-repo/gitv.git
cd gitv
cargo build --release
# Add to PATH or use directly
```

### Pre-built Binaries

Download from [releases](https://github.com/your-repo/gitv/releases)

## Configuration

GitV uses Git config to store settings. Configure your LLM provider:

```bash
# Set global provider (e.g., Google Gemini)
git config --global gitv.defaultprovider google
git config --global gitv.google-apikey "your-api-key"
git config --global gitv.google-model "gemini-1.5-pro"

# Or set locally for a project
git config --local gitv.defaultprovider google
git config --local gitv.google-apikey "your-api-key"
git config --local gitv.google-model "gemini-1.5-flash"
```

Supported providers: `openai`, `anthropic`, `google`, `cohere`, `groq`, `ollama`, etc.

You can also use the config command:

```bash
# Set provider, API key, and model
gitv config --provider google --api-key "your-key" --model "gemini-1.5-pro"

# For project-specific config
gitv config --project --provider google --model "gemini-1.5-flash"
```

## How to Use

### Generating Commit Messages

```bash
# Stage your changes
git add .

# Generate a commit message
gitv commit

# Or specify a custom instruction
gitv commit --instructions "Focus on the API changes"
```

### Creating Pull Requests

```bash
# Create a PR description for current branch vs main
gitv pr

# Specify branches
gitv pr --base main --head feature-branch

# Use different detail level
gitv pr --detail-level detailed
```

### Code Reviews

```bash
# Review changes in current branch
gitv review

# Review specific branches
gitv review --base main --head feature-branch

# Review with custom instructions
gitv review --instructions "Check for security issues"
```

### Generating Changelogs

```bash
# Generate changelog from commits
gitv changelog

# Specify version and detail level
gitv changelog --version 1.2.0 --detail-level standard

# Generate release notes
gitv release-notes --version 1.2.0
```

### Managing Configuration

```bash
# View current config
gitv config

# Set provider and API key
gitv config --provider openai --api-key "your-key"

# Configure project-specific settings
gitv config --project --instructions "Use conventional commits"
```

### Other Commands

```bash
# List available instruction presets
gitv list-presets

# Generate a single message (not a commit)
gitv msg --instructions "Summarize this change"

# Analyze project metadata
gitv project

# Serve as MCP server
gitv serve

# Wire operations (caching, syncing)
gitv wire
```

## Examples

### First Time Setup

```bash
# Install GitV
cargo install --git https://github.com/your-repo/gitv.git

# Configure Google Gemini
git config --global gitv.defaultprovider google
git config --global gitv.google-apikey "your-api-key"

# Make your first AI commit
git add .
gitv commit
```

### Project-Specific Configuration

```bash
cd my-project
git config --local alias.instructions "Use conventional commits format"
git config --local alias.useemoji true
```

### Advanced Usage

```bash
# Generate PR with high detail
gitv pr --detail-level detailed --instructions "Include testing notes"

# Review with security focus
gitv review --instructions "Check for SQL injection vulnerabilities"

# Custom changelog
gitv changelog --preset "detailed" --instructions "Focus on breaking changes"
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under the MIT License. See [LICENSE.md](LICENSE.md) for details.
