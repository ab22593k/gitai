# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
