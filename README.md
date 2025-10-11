# Git With AI

AI-powered Git toolkit that enhances workflow with intelligent commit messages, pull request generation, code reviews, changelogs, and more. It integrates with various LLM providers to automate and improve your development process.

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
git clone https://github.com/ab22593k/gitai.git
cd gitai
cargo build --release
```

## Configuration

gitai uses Git config to store settings. Configure your LLM provider:

```bash
# Set --global/local provider (e.g., Google Gemini)
git config --global gitai.defaultprovider google
git config --global gitai.google-apikey "your-api-key"
git config --global gitai.google-model "gemini-1.5-pro"
```

Supported providers: `openai`, `anthropic`, `google`, `cohere`, `groq`, `ollama`, etc.

You can also use the config command:

## How to Use

### Generating Commit Messages

```bash
# Stage your changes
git add .

# Generate a commit message
git message

# Or specify a custom instruction
git message --instructions "Focus on the API changes"
```

### Generating Changelogs

```bash
# Generate changelog from commits
git changelog

# Specify version and detail level
git changelog --version 1.2.0 --detail-level standard

# Generate release notes
git release-notes --version 1.2.0
```
### Managing Configuration


### Other Commands

```bash
# Serve as MCP server
git serve

# Wire operations (caching, syncing)
git wire
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Licensed under the MIT License. See [LICENSE.md](LICENSE.md) for details.
