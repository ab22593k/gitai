# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.1.4] - 2026-02-27

### ✨ Added

- Automate changelog reference resolution via libgit2's describe functionality and introduce a --save flag for persistent updates to CHANGELOG.md. (31947feb395596b3d132ebf1e60d7c3c9bd14e8c)
- Enforce 'Principal Linux Kernel Maintainer' persona for LLM prompts, requiring structured Problem/Solution/Reasoning narratives with strict 72/82 character line wrapping. (7cda26121e28603a42d70073834d28e86085fb0d)
- Integrate OpenTelemetry tracing across application entry points and core LLM modules to monitor request flow and performance bottlenecks. (ac877320139a79d4b5e95697ef45382f45b0634d)
- Add support for Cerebras as an LLM provider within the core LLM and configuration modules. (a03a633016816879a01a2e389ca631f588ba984e)
- Render code diffs with syntax highlighting in the TUI context selection screen to improve review ergonomics. (fdf73bc090eea96ec28d3abbdd8ff8adc4ea9204)
- Introduce --model CLI parameter to allow explicit override of the configured LLM model for specific operations. (377232c5d67d43828839fb20952c9c5ab3eb19a0)

### 🔄 Changed

- Offload synchronous libgit2 operations to tokio::task::spawn_blocking and utilize JoinSet for concurrent remote synchronization to prevent TUI executor starvation. (a37ebfaaff9a6b740cf71dc60111e91bbc045666)
- Overhaul TUI theme with indigo gradients and Nerd Font icons; replace syntect with native Ratatui styling to reduce rendering overhead. (119156906137c02077079fdc1bd5fe9dc398f9e7, c5b7d1206a7180d09ee6f0ceace6d21a8b4f2c90)
- Implement automated diff truncation in prompt generation to mitigate LLM context window overflow. (bc8ca39d074b332cf14204ee82c51dd53d68b767)

### 🗑️ Removed

- Remove legacy agent skill documentation and provider-specific symlinks to simplify project surface area. (22c6a38eb1486141c6fbf36b8ec2b2a6867b033a)
- Delete specialized token_optimizer and semantic_similarity modules, consolidating their logic into core context and prompt management services. (4d73083448216fbdd35934435abc1b319ddca131, 252047af4c8e49b3c8c903cebfd1f5572b54f2be)

### ⚠️ Breaking Changes

- Rename project from 'gait' to 'gitai', requiring updates to configuration files, environment variables, and CI/CD pipelines to match the new naming convention. (1573c5df3603fc451e108b52fa56b953dc3affed)
- Remove 'default_provider' from global configuration, standardizing on explicit provider selection or internal defaults. (bf799f80041ce5d32aee09f987f0f1ecaec84782)
- Deprecate standalone 'keys' binary and integrate API key management directly into the core configuration subsystem. (e2f0bb74770a6d41c940e013856aff13d74ce502)

### 📊 Metrics

- Total Commits: 34
- Files Changed: 267
- Insertions: 7928
- Deletions: 10674

<!-- -------------------------------------------------------------- -->

## [v0.1.3] -

### ✨ Added

- Introduce a dedicated TUI theme management system for consistent styling (c17c2d3)
- Add `--debug-llm` command-line flag to dump raw Language Model prompts (356a53d)
- Provide essential system instructions for the Claude Large Language Model (f54ddf1)

### 🔄 Changed

- Standardize overall application structure, update core dependencies, and refine continuous integration workflow (c4200f6)
- Refine core application architecture, streamline configuration application, and enhance Language Model integration (e320e88)
- Improve efficiency and correctness of token optimization logic and performance (3607039, 8f32e21)
- Enhance core context data processing and refine Git data access and integration into the application's context (8f32e21)
- Refine internal logic for generating commit messages and streamline Language Model prompts (5e3c2eb)
- Streamline TUI application state management and refine user input processing for improved user experience (1d71e0e)
- Enhance TUI responsiveness and integrate more robustly with the commit generation service (1d71e0e)
- Refine TUI application main loop, improving event handling, state integration, and code organization (f26bd7d)
- Improve TUI visual feedback mechanisms, including loading indicators, and refine overall rendering (19748cb)
- Update and refine agent system instructions documentation in `AGENTS.md` for improved clarity and guidance (660ed48, b3d222a, ca83eb4, f54ddf1)
- Improve structure, clarity, and reliability of configuration test suite (8c402f1)
- Update various test suites to reflect changes in core application logic and components (c4200f6, e320e88, 5e3c2eb, 3607039)

### 📊 Metrics

- Total Commits: 15
- Files Changed: 72
- Insertions: 2804
- Deletions: 663

## [0.1.1] -

### Added

- Add `CHANGELOG.md` to document project evolution and release notes (f64fa5)

### Changed

- Consolidate file analysis logic into the `analyzer` module, moving implementations from `src/file_analyzers/` and updating import paths (1a259d)
- Update project version in `Cargo.toml` from 0.1.0 to 0.1.1 (f64fa5)
- Adjust `README.md` header from 'Wire operations (caching, syncing)' to 'Wire operations (syncing)' (f64fa5)

### Removed

- Remove the `git-serve` command and all related server modules and files, including `src/bin/serve.rs` and the entire `src/server` directory (d87fc5)
- Remove unused `use_emoji` and `instruction_preset` configurations from `Config` and delete `src/instruction_presets.rs` (077f09)
- Remove `EditingUserInfo` mode and related handling from the TUI, simplifying its state and input handling (2bfc62)
- Remove unused `use` statements in `src/config.rs` to improve code readability (0e16c3)

### ⚠ Breaking Changes

- The `git-serve` command and its associated server functionality have been removed, making it unavailable for use (d87fc5)

### 📊 Metrics

- Total Commits: 6
- Files Changed: 73
- Insertions: 2129
- Deletions: 4879
