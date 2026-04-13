# ADR-0003: Terminal TUI over GUI for Interactive Workflows

**Date:** 2026-02-15  
**Status:** Accepted

## Context

GitAI's interactive workflows (context selection, message editing, review)
need a user interface. Options considered:

- **Terminal TUI** (ratatui + crossterm) — runs anywhere SSH works
- **Desktop GUI** (egui, iced, Flutter) — richer visuals, requires distribution
- **Web UI** (Axum + HTMX/React) — accessible, adds deployment complexity

## Decision

Build a terminal TUI using `ratatui` with `crossterm` backend.

## Consequences

### Positive
- Zero installation friction on servers — works over SSH
- Fast startup, low resource usage
- Native integration with developer workflows (tmux, terminal multiplexers)
- Theme system supports dark/light/system modes with adaptive colors

### Negative
- Limited visual expressiveness (no images, complex layouts)
- Platform-specific terminal quirks (emoji rendering, mouse support)
- Harder to onboard non-technical users

### Future Revisit Triggers
- A significant user base requests a web or desktop interface.
- TUI limitations block a critical feature (e.g., rich diff review).
