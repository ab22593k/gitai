# ADR-0004: Enum-Based Provider Dispatch over Trait Objects

**Date:** 2026-04-09  
**Status:** Accepted

## Context

GitAI supports 9+ LLM providers (Google, OpenAI, Anthropic, Groq, etc.).
The initial implementation used `Box<dyn ModelProvider>` with the `async_trait`
crate for dynamic dispatch. This pattern has two problems:

1. **Dyn incompatibility with native async traits** — Rust 2024's native
   `async fn` in traits is not object-safe, preventing removal of `async_trait`.
2. **Scattered provider identity** — provider names existed as `&str` literals
   across 7 files (37 instances), making additions error-prone.

## Decision

Replace `Box<dyn ModelProvider>` with a `ProviderKind` enum that provides:
- Single source of truth for provider identity, names, and defaults
- Static dispatch via `match` (zero vtable overhead)
- Full compatibility with native Rust 2024 async functions
- Bridge to the external `llm` crate's `LLMBackend` enum

The enum lives in `src/llm/provider.rs` and is the *only* place provider
string literals should appear.

## Consequences

### Positive
- Adding a provider requires changes in exactly one file (`provider.rs`)
- Zero `async_trait` dependency in GitAI code (removed from Cargo.toml)
- Compile-time exhaustiveness checking on provider match arms
- No heap allocation for provider dispatch

### Negative
- Adding a provider requires a new enum variant (not just a config entry)
- `LLMBackend` mapping needed for backends not yet in the external `llm` crate
  (e.g., Cerebras → OpenRouter fallback)

### Future Revisit Triggers
- The external `llm` crate adds Cerebras/Xai backends natively.
- Plugin architecture demands runtime provider registration (unlikely for GitAI's scope).
