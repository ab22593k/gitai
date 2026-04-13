# ADR-0002: Layered Configuration (env → local git → global git)

**Date:** 2026-02-01  
**Status:** Accepted

## Context

Users need to configure GitAI at three scopes:
1. **Machine-wide** — one API key and model for all repos
2. **Per-project** — different models or instructions per repo
3. **CI/one-shot** — override via environment variables without touching git config

## Decision

Configuration layers are resolved in priority order:
```
Environment variables > Local git config (.git/config) > Global git config (~/.gitconfig)
```

Each setting is checked at each layer; the first match wins. API keys are read
from env vars (e.g., `GOOGLE_API_KEY`) or git config keys (`gitai.google-apikey`).

Project configs (`save_as_project_config`) strip API keys before writing to
`.git/config` — a security invariant enforced at the merge boundary.

## Consequences

### Positive
- No secrets in `.git/config` (which may be shared or backed up insecurely)
- CI/CD can override everything via env vars without repo changes
- Users set API keys once globally; projects customize models/instructions

### Negative
- Debugging "why is this value set?" requires checking three sources
- No built-in visibility into which layer resolved a value (improvement opportunity)

### Future Revisit Triggers
- Add a `--dry-run` or `--show-config` flag that prints the resolution chain
  for each config key (see AI-first assessment, improvement #3).
