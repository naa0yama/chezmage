# Project Summary

- Think in English, explain, and respond to chat in Japanese.
- Use half-width brackets instead of full-width brackets in the Japanese explanations output.
- When writing Japanese and half-width alphanumeric characters or codes in one sentence, please enclose the half-width alphanumeric characters in backquotes and leave half-width spaces before and after them.

## Commands

All tasks use `mise run <task>`:

| Task                  | Command                       |
| --------------------- | ----------------------------- |
| Build                 | `mise run build`              |
| Test                  | `mise run test`               |
| TDD watch             | `mise run test:watch`         |
| Doc tests             | `mise run test:doc`           |
| Format                | `mise run fmt`                |
| Format check          | `mise run fmt:check`          |
| Lint (clippy)         | `mise run clippy`             |
| Lint strict           | `mise run clippy:strict`      |
| AST rules             | `mise run ast-grep`           |
| Pre-commit (required) | `mise run pre-commit`         |
| Coverage              | `mise run coverage`           |
| Deny (licenses/deps)  | `mise run deny`               |
| Build (OTel)          | `cargo build --features otel` |

## Commit Convention

Conventional Commits: `<type>: <description>` or `<type>(<scope>): <description>`

Allowed types: feat, update, fix, style, refactor, docs, perf, test, build, ci, chore, remove, revert

## Workflow

1. Write tests (for new features / bug fixes)
2. Implement
3. Run `mise run test` — all tests must pass
4. Stage only the relevant files
5. Run `mise run pre-commit` (runs fmt:check, clippy:strict, ast-grep)
6. If errors, fix → re-stage → re-run `mise run pre-commit`

## Code Comments

- Write all code comments (doc comments, inline comments) in concise English.

## Key Coding Rules

- **Imports**: All `use` statements at file top level, grouped: `std` -> external crates -> `crate`/`super`. No wildcards (`*`). Aliases (`as`) permitted for name conflicts and re-exports.
- **Error handling**: Never use bare `?`. Always add `.context()` or `.with_context()`.
- **Logging**: Use `tracing` crate, not `println!` / `dbg!`. For container/OTel support, build with `--features otel` and set `OTEL_EXPORTER_OTLP_ENDPOINT` env var.
- **Tests**: Arrange / Act / Assert pattern. Unit tests in `#[cfg(test)] mod tests`, integration tests in `tests/`. `#![allow(clippy::unwrap_used)]` is permitted in test code.
- See [docs/project_rules.md](./docs/project_rules.md) for full details.

## Skill Maintenance

- When modifying coding rules, workflow, or project conventions in `CLAUDE.md` or `docs/project_rules.md`, also update the corresponding `.claude/skills/` files to keep them in sync.
