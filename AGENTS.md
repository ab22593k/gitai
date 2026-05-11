# AGENTS.md

## Build prerequisites

`git2` uses vendored-openssl + vendored-libgit2 (C compilation). Build fails without:

- C compiler (gcc/g++)
- cmake
- openssl-devel / openssl-dev
- pkg-config
- perl

Fedora: `dnf install -y gcc gcc-c++ make pkgconfig openssl-devel cmake perl`
Ubuntu: `apt install -y gcc g++ make pkg-config libssl-dev cmake perl`

## Workspace layout

```
claw-core          → library crate (ALL business logic lives here)
claw-message       → git-message  (thin CLI wrapper, ~50 lines)
claw-pr            → git-pr       (thin CLI wrapper)
claw-changelog     → git-changelog (thin CLI wrapper)
claw-notes         → git-notes   (thin CLI wrapper)
claw-wire          → git-wire    (thin CLI wrapper)
```

All 5 binaries depend on `claw-core` only. Binary names differ from crate names.
When adding functionality, edit `claw-core` — the binary crates are just arg parsers.

## Verification commands

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

CI runs them in that order. All must pass.

## Lint rules that bite

- `unwrap_used` = **deny** — use `.context()?`, `anyhow!`, or explicit error handling
- `unsafe_code` = **forbid**
- `clippy::pedantic` = **deny** (with targeted allows for `missing_errors_doc`, `module_name_repetitions`, etc.)
- `enum_glob_use` = **deny**
- Cast lints (`cast_possible_truncation`, `cast_sign_loss`, `cast_precision_loss`) = **warn**
- `as_conversions` = **warn**

## Known discrepancies

- **README says 5 LLM providers**; code only implements 2: `Google` and `OpenRouter`. The `ProviderKind` enum in `crates/claw-core/src/llm/provider.rs` is the single source of truth for provider identity.
- **README says `.gitwire.toml`**; the actual config file is `.gitwire` (git-config syntax, not TOML). Parsed via `git2::Config::open()`.
- **`integration` feature flag** exists in `claw-core/Cargo.toml` but no code gates on it. `cargo test --features integration` is identical to `cargo test`.

## Architecture pointers

Key design decisions documented in `docs/`:

- `0001` — libgit2 over gitoxide (vendored build tradeoff)
- `0002` — Config layering: env vars > local git config > global git config
- `0004` — Enum-based provider dispatch (`ProviderKind`), not trait objects

## Running a single binary

```sh
cargo run --bin git-message -- --print
cargo run --bin git-pr -- --from main --print
cargo run --bin git-changelog -- --from v0.1.3 --to HEAD
cargo run --bin git-notes -- --from v0.1.3 --to v0.1.4
cargo run --bin git-wire -- sync --url <repo> --rev main --src lib --dst vendor/lib
```

## Testing a single crate

```sh
cargo test -p claw-core
cargo test -p claw-message
```

### Rules:

- Every complex function must include a `why` comment explaining the business logic or external dependency that justified its implementation
- Structure every response to end with concrete forward-looking suggestions.
