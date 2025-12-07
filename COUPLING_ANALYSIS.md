# Tight Coupling Analysis for Gait Codebase

## Executive Summary

After analyzing the codebase, I've identified **5 major areas of tight coupling** that could benefit from refactoring. The codebase generally follows good practices, but there are opportunities to improve separation of concerns, testability, and maintainability.

---

## 1. ✅ COMPLETED: CommitService ↔ CompletionService Code Duplication

**Location:** `src/features/commit/service.rs` and `src/features/commit/completion.rs`

**Issue:** These two services shared significant duplicated code (~100 lines).

**Solution Applied:**
- Created new `git_service_core.rs` with `GitServiceCore` struct
- Centralized common functionality:
  - `perform_commit()` - ~50 lines consolidated
  - `is_remote_repository()`
  - `check_environment()`
  - `get_git_info()` and `get_git_info_with_unstaged()`
- Both `CommitService` and `CompletionService` now delegate to `GitServiceCore`

**Files Changed:**
- `src/features/commit/git_service_core.rs` (NEW - 179 lines)
- `src/features/commit/service.rs` (refactored to use GitServiceCore)
- `src/features/commit/completion.rs` (refactored to use GitServiceCore)
- `src/features/commit/mod.rs` (added new module)

**Lines Reduced:** ~100 lines of duplicated code eliminated

---

## 2. ✅ COMPLETED: GitRepo God Object Split

**Location:** `src/git/repository.rs`

**Issue:** GitRepo was ~960 lines with 42 methods handling too many responsibilities.

**Solution Applied:**
- Created `src/git/hooks.rs` - Git hook execution functionality (85 lines)
- Created `src/git/history.rs` - Commit history operations (235 lines)
- `repository.rs` now delegates to these modules
- Repository was reduced from 958 lines to ~750 lines

---

## 2. Config Module Coupled to LLM and Git

**Location:** `src/config.rs`

**Issue:** Config directly imports and depends on:
- `crate::core::llm::*` - LLM provider functions
- `crate::git::GitRepo` - Git repository checking

**Evidence (lines 1-4):**
```rust
use crate::core::llm::{
    get_available_provider_names, get_default_model_for_provider, provider_requires_api_key,
};
use crate::git::GitRepo;
```

**Problem:** 
- Config should be a pure data structure with minimal dependencies
- Testing Config requires mocking LLM and Git modules
- Config is responsible for both data storage AND environment validation (violates SRP)

**Recommendation:**
- Split `Config` into `ConfigData` (pure struct) and `ConfigValidator` (validation logic)
- Use dependency injection for LLM provider info
- Move `check_environment()` to a separate validation module

**Priority:** MEDIUM - Improves testability and separation of concerns

---

## 3. TuiCommit Directly Depends on Concrete Service Types

**Location:** `src/tui/app.rs`

**Issue:** `TuiCommit` takes concrete `Arc<CommitService>` and `Arc<CompletionService>` instead of traits.

**Evidence (lines 26-30):**
```rust
pub struct TuiCommit {
    // Direct dependency on concrete types
    service: Arc<CommitService>,
    completion_service: Arc<CompletionService>,
    // ...
}
```

**Problem:**
- Hard to test TUI logic without real services
- Can't swap implementations
- Tight coupling between presentation and business logic

**Recommendation:** Define service traits:
```rust
pub trait MessageGenerator: Send + Sync {
    async fn generate_message(&self, instructions: &str) -> Result<GeneratedMessage>;
}

pub trait MessageCompleter: Send + Sync {
    async fn complete_message(&self, prefix: &str, context_ratio: f32) -> Result<GeneratedMessage>;
}

pub trait Committer: Send + Sync {
    fn perform_commit(&self, message: &str, amend: bool, commit_ref: Option<&str>) -> Result<CommitResult>;
}
```

**Priority:** MEDIUM - Improves testability significantly

---

## 4. CLI Module Creates Services Directly

**Location:** `src/features/commit/cli.rs`

**Issue:** CLI functions create services directly using concrete constructors:
- `create_commit_service()` (lines 533-567)
- `create_completion_service()` (lines 569-595)

**Evidence:**
```rust
pub fn create_commit_service(
    common: &CommonParams,
    repository_url: Option<String>,
    config: &Config,
) -> Result<Arc<CommitService>> {
    // Direct construction without abstraction
    let git_repo = GitRepo::new_from_url(repository_url)?;
    let service = CommitService::new(...)?;
    Ok(Arc::new(service))
}
```

**Problem:**
- Hard to test CLI logic in isolation
- Service creation coupled to CLI layer
- No factory abstraction for different environments

**Recommendation:** Introduce a ServiceFactory:
```rust
pub trait ServiceFactory {
    fn create_commit_service(&self, common: &CommonParams) -> Result<Arc<dyn MessageGenerator>>;
    fn create_completion_service(&self, common: &CommonParams) -> Result<Arc<dyn MessageCompleter>>;
}
```

**Priority:** MEDIUM

---

## 5. GitRepo is a God Object

**Location:** `src/git/repository.rs` (~960 lines, 42 methods)

**Issue:** `GitRepo` does too many things:
- Repository discovery/cloning
- Staged file handling
- Commit context building
- Hook execution
- Branch comparison
- Remote repository handling
- Commit caching
- Author history lookup

**Evidence:** The file has 42 methods spanning nearly 1000 lines.

**Problem:**
- Hard to understand and maintain
- Violates Single Responsibility Principle
- Testing is difficult due to many responsibilities

**Recommendation:** Split into focused components:
```
git/
├── repository.rs      # Core repo operations (open, clone, is_remote)
├── staged_files.rs    # Staged file operations
├── commit_context.rs  # Context building for AI
├── hooks.rs           # Hook execution
├── branch_ops.rs      # Branch comparison/diff
├── history.rs         # Commit history and author history
└── mod.rs             # Facade that composes all
```

**Priority:** HIGH - The largest refactoring opportunity

---

## 6. Prompt Functions Tightly Coupled to Config

**Location:** `src/features/commit/prompt.rs`

**Issue:** Prompt creation functions take `Config` but only use `instructions`:
```rust
pub fn create_system_prompt(config: &Config) -> anyhow::Result<String> {
    let combined_instructions = get_combined_instructions(config);
    // Only uses config.instructions
}
```

**Problem:**
- Over-specified dependency
- Hard to test with just instructions

**Recommendation:** Take only what's needed:
```rust
pub fn create_system_prompt(instructions: &str) -> anyhow::Result<String>
```

**Priority:** LOW - Minor improvement

---

## Dependency Graph (Current)

```
                    ┌────────────────┐
                    │     app.rs     │
                    └───────┬────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
        ▼                   ▼                   ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│ features/     │   │ features/     │   │    tui/       │
│ commit/       │   │ changelog/    │   │               │
└───────┬───────┘   └───────┬───────┘   └───────┬───────┘
        │                   │                   │
        └───────────────────┼───────────────────┘
                            │
                    ┌───────┴───────┐
                    │               │
                    ▼               ▼
            ┌───────────┐   ┌───────────┐
            │  config   │◄──│   git/    │
            └─────┬─────┘   └─────┬─────┘
                  │               │
                  └───────┬───────┘
                          │
                          ▼
                  ┌───────────────┐
                  │   core/llm    │
                  └───────────────┘
```

**Issue:** Circular-ish dependency between config → llm and git modules.

---

## Recommended Refactoring Priority

1. **HIGH:** Extract common code from CommitService/CompletionService
2. **HIGH:** Split GitRepo into smaller, focused modules  
3. **MEDIUM:** Introduce service traits for TUI testing
4. **MEDIUM:** Decouple Config from LLM/Git modules
5. **LOW:** Simplify prompt function signatures

---

## Quick Wins (Can Fix Now)

1. **Extract `perform_commit()` to shared module** - 30 min
2. **Create `GitServiceBase` trait** - 1 hour
3. **Pass `&str` instead of `&Config` to prompt functions** - 15 min
4. **Split `GitRepo` hook execution to separate file** - 1 hour

---

## Files Modified for Each Refactor

| Refactor | Files Affected |
|----------|---------------|
| CommitService/CompletionService | `service.rs`, `completion.rs`, new `base_service.rs` |
| Config decoupling | `config.rs`, `core/llm.rs`, new `config_validator.rs` |
| Service traits | `tui/app.rs`, `service.rs`, `completion.rs`, new traits module |
| GitRepo split | `git/repository.rs` → 5-6 new files |
