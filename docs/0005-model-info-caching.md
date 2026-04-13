# ADR-0005: Singleton ModelInfoService with In-Memory Caching

**Date:** 2026-03-01  
**Status:** Accepted

## Context

LLM providers expose different context window sizes via their APIs (Google,
Groq, OpenRouter). GitAI needs to know these sizes to truncate diffs
appropriately and avoid context window overflows. Querying the API on every
request would be slow and wasteful.

## Decision

Implement `ModelInfoService` as a global singleton (`OnceLock`) with:
- **In-memory cache** keyed by `provider:model` with 1-hour TTL
- **Read-write lock** (`tokio::sync::RwLock`) for concurrent access
- **Fallback chain**: known model names → provider defaults → 8,192 global floor
- **Enum-based dispatch** via `ProviderKind` (see ADR-0004)

Only providers with public model info APIs (Google, Groq, OpenRouter) perform
live fetches. Others use fallback limits directly.

## Consequences

### Positive
- First request pays the API cost; subsequent requests are instant
- Thread-safe concurrent access without blocking
- Graceful degradation when APIs are unreachable

### Negative
- Cache is process-scoped — multiple binaries don't share cached values
- Stale data if provider changes model specs within the TTL window
- `OnceLock` singleton makes testing harder (tests must create fresh instances)

### Future Revisit Triggers
- Cross-process caching needed (could add a file-based cache or Redis).
- Model spec changes become frequent enough to warrant shorter TTLs or
  push-based invalidation.
