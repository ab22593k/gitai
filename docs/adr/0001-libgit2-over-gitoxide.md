# ADR-0001: Use libgit2 (git2 crate) for Git operations

**Date:** 2026-02-01  
**Status:** Accepted

## Context

GitAI needs to read Git history, staged files, branch information, and commit
data to build context for LLM prompts. At project inception, two Rust options
existed:

- **`git2`** — Safe Rust bindings to libgit2 (C library). Mature, feature-complete.
- **`gitoxide` (gix)** — Pure Rust implementation. Growing but incomplete at the time.

## Decision

Use the `git2` crate with vendored OpenSSL and libgit2 (`vendored-openssl`,
`vendored-libgit2` features).

## Consequences

### Positive
- Full Git API coverage from day one (remotes, tags, diffs, blame)
- Stable, well-documented API with predictable behavior
- Vendored builds ensure reproducible CI/CD without system dependencies

### Negative
- Adds C compilation step (OpenSSL, libgit2) to build times (~2-3 min extra)
- Binary size increase (~5-8 MB from vendored libs)
- Not pure Rust (some teams prefer this)

### Future Revisit Triggers
- `gitoxide` achieves full feature parity *and* the git2 build-time burden
  becomes a measurable CI bottleneck.
- A security audit flags libgit2 as a risk surface.
