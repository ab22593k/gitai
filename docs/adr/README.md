# Architecture Decision Records

This directory documents the major architectural decisions that shape GitAI.
Each record captures the context, the decision, and the consequences so that
future contributors (human or AI) can understand *why* the codebase is the way
it is without reverse-engineering from implementation.

## Records

| #    | Title                                                    | Date       |
|------|----------------------------------------------------------|------------|
| 0001 | [Use libgit2 (git2 crate) for Git operations](0001-libgit2-over-gitoxide.md)   | 2026-02-01 |
| 0002 | [Layered configuration (env → local git → global git)](0002-config-layering.md) | 2026-02-01 |
| 0003 | [Terminal TUI over GUI for interactive workflows](0003-tui-over-gui.md)        | 2026-02-15 |
| 0004 | [Enum-based provider dispatch over trait objects](0004-provider-enum-dispatch.md)| 2026-04-09 |
| 0005 | [Singleton ModelInfoService with in-memory caching](0005-model-info-caching.md)  | 2026-03-01 |
