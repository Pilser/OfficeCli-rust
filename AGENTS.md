# AGENTS.md

Sub-agent rules for Phase 3 implementation.

## Build Rules for Sub-Agents

Sub-agents MUST ONLY use these two build commands:

```bash
cargo check                    # Fast compile check (always run first)
cargo test -p <crate-name>     # Run unit tests for the specific crate
```

**NEVER run** `cargo build --release` — the user handles release builds manually to avoid filling storage.

**NEVER run** `cargo clippy`, `cargo fmt`, or `cargo build` (debug) — these are not needed for correctness verification.

## Before Coding

1. **Web search first** — If the task involves new dependencies or unfamiliar APIs, use `websearch` or `webfetch` to read docs before writing any code.
2. **Read existing code** — Understand the pattern by reading analogous files in the codebase before implementing.

## Verification Flow

1. `cargo check` — must pass with 0 errors
2. `cargo test -p <crate-name>` — must pass for the crate being modified
3. Report back: what was created/changed, whether both commands passed, and any remaining errors


