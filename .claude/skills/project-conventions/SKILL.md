---
name: project-conventions
description: >-
  Project-specific conventions for the dtvmgr Rust CLI. Enforces mandatory
  error context on all ? operators, tracing-only logging, strict import
  grouping, mise-based tooling, and 6 custom ast-grep rules. Use when
  writing, reviewing, or modifying .rs files, running builds/tests, or
  creating commits. Complements rust-implementation with project-specific rules.
license: AGPL-3.0
---

# Project Conventions — dtvmgr

## 1. Mandatory Error Context

Every `?` operator MUST have `.context()` or `.with_context()` attached.
Bare `?` is forbidden. Enforced by ast-grep rule `error-context-required`.

```rust
// Good
let url = Url::parse(s).context("invalid URL")?;
let val = u32::try_from(n).with_context(|| format!("overflow: {n}"))?;

// Bad — bare ?
let url = Url::parse(s)?;
```

## 2. Logging: tracing Only

`println!`, `eprintln!`, and `dbg!` are forbidden in production code.
Use `tracing` macros with structured fields.

```rust
tracing::info!(page = page, total = total, "fetched programs");
tracing::warn!(?err, "retry after failure");
```

Exception: `build.rs` may use `println!` with `// ast-grep-ignore: no-println-debug`.
Also enforced by clippy lints `print_stdout`, `print_stderr`, `dbg_macro`.

## 3. Import Grouping

All `use` statements at file top level, grouped with blank-line separators:

```rust
// 1. std
use std::sync::Arc;

// 2. External crates
use anyhow::{Context as _, Result};
use tokio::sync::Mutex;

// 3. crate / super
use crate::libs::syoboi::types::SyoboiProgram;
```

Wildcards (`*`) are forbidden except `use super::*` in test modules.
Aliases (`as`) are permitted for name conflicts and re-exports.

## 4. Commands: mise Only

Never run `cargo` directly. All tasks go through `mise run`:

| Task            | Command                             |
| --------------- | ----------------------------------- |
| Build           | `mise run build`                    |
| Test            | `mise run test`                     |
| TDD watch       | `mise run test:watch`               |
| Doc tests       | `mise run test:doc`                 |
| Format          | `mise run fmt`                      |
| Format check    | `mise run fmt:check`                |
| Lint (clippy)   | `mise run clippy`                   |
| Lint strict     | `mise run clippy:strict`            |
| AST rules       | `mise run ast-grep`                 |
| Pre-commit      | `mise run pre-commit`               |
| Coverage        | `mise run coverage`                 |
| Deny            | `mise run deny`                     |
| Build with OTel | `mise run build -- --features otel` |

## 5. Workflow

1. Write tests first (for new features / bug fixes)
2. Implement the feature
3. Run `mise run test` — all tests must pass
4. Stage only the relevant files
5. Run `mise run pre-commit` (runs `fmt:check`, `clippy:strict`, `ast-grep`)
6. If errors, fix, re-stage, and re-run `mise run pre-commit`

## 6. Code Comments

All code comments (doc comments, inline comments) must be in concise English.
Japanese is forbidden in source code.

## 7. Commit Convention

Conventional Commits format: `<type>: <description>` or `<type>(<scope>): <description>`

Allowed types: `feat`, `update`, `fix`, `style`, `refactor`, `docs`, `perf`,
`test`, `build`, `ci`, `chore`, `remove`, `revert`

## 8. No Blocking I/O in Async

In `async fn`, synchronous blocking calls are forbidden (severity: **error**).
Enforced by ast-grep rule `no-blocking-in-async`.

| Forbidden               | Use instead               |
| ----------------------- | ------------------------- |
| `std::fs::*`            | `tokio::fs::*`            |
| `std::thread::sleep`    | `tokio::time::sleep`      |
| `std::net::*`           | `tokio::net::*`           |
| `std::process::Command` | `tokio::process::Command` |

## 9. Reference Files

| Topic                    | File                                         |
| ------------------------ | -------------------------------------------- |
| ast-grep rules (6 rules) | `references/ast-grep-rules.md`               |
| Testing patterns         | `references/testing-patterns.md`             |
| Module & project layout  | `references/module-and-project-structure.md` |
