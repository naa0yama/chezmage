---
name: project-conventions
description: >-
  Project-specific conventions for the chezmage Rust CLI. Overrides and extends
  the shared rust-project-conventions skill. Defines mise commands, OTel config,
  Miri skip categories, and project source layout. Use when writing, reviewing,
  or modifying .rs files, running builds/tests, or creating commits.
---

# Project Conventions — chezmage

> **Base rules**: See `~/.claude/skills/rust-project-conventions/SKILL.md` for
> mandatory error context, tracing-only logging, import grouping, workflow,
> code comments, commit convention, and blocking I/O rules. This file only
> documents chezmage-specific overrides and additions.

## Commands: mise Only

Never run `cargo` directly. All tasks go through `mise run`:

| Task            | Command                             |
| --------------- | ----------------------------------- |
| Build           | `mise run build`                    |
| Test            | `mise run test`                     |
| TDD watch       | `mise run test:watch`               |
| Doc tests       | `mise run test:doc`                 |
| Trace test      | `mise run test:trace`               |
| Format          | `mise run fmt`                      |
| Format check    | `mise run fmt:check`                |
| Lint (clippy)   | `mise run clippy`                   |
| Lint strict     | `mise run clippy:strict`            |
| AST rules       | `mise run ast-grep`                 |
| Pre-commit      | `mise run pre-commit`               |
| Coverage        | `mise run coverage`                 |
| Deny            | `mise run deny`                     |
| Build with OTel | `mise run build -- --features otel` |
| O2 (start)      | `mise run o2`                       |
| O2 (stop)       | `mise run o2:stop`                  |

## Reference Files

| Topic                      | File                                                                       |
| -------------------------- | -------------------------------------------------------------------------- |
| Testing patterns & Miri    | `references/testing-patterns.md`                                           |
| Project source layout      | `references/module-and-project-structure.md`                               |
| Module structure (shared)  | `~/.claude/skills/rust-project-conventions/references/module-structure.md` |
| ast-grep rules (shared)    | `~/.claude/skills/rust-project-conventions/references/ast-grep-rules.md`   |
| Testing templates (shared) | `~/.claude/skills/rust-coding/references/testing-templates.md`             |
