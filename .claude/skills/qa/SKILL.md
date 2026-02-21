---
name: qa
description: >-
  Quality assurance agent that runs pre-commit checks, categorizes errors,
  and applies fixes in a cycle until all checks pass. Use /qa after coding
  to ensure code quality before committing.
---

# QA Agent — Quality Check & Fix Cycle

## Prerequisites

Before starting, be familiar with:

- `/app/.claude/skills/project-conventions/references/ast-grep-rules.md` — 6 custom lint rules
- `/app/.claude/skills/coding/references/common-errors.md` — frequent errors and fixes

## Workflow

### Step 1: Auto-format

Run `mise run fmt` to fix all auto-fixable formatting issues first.
Stage the formatted files.

### Step 2: Clippy Strict

Run `mise run clippy:strict` and categorize warnings:

| Category       | Examples                     | Action                             |
| -------------- | ---------------------------- | ---------------------------------- |
| Error handling | `unwrap_used`, `expect_used` | Add `.context()`/`.with_context()` |
| Logging        | `print_stdout`, `dbg_macro`  | Replace with `tracing`             |
| Naming         | `module_name_repetitions`    | Add `#[allow(...)]` or rename      |
| Type safety    | `as_conversions`             | Use `try_from`/`try_into`          |
| Style          | `needless_pass_by_value`     | Fix per suggestion                 |

Fix each warning. If unsure about a fix, read `project-conventions/SKILL.md`.

### Step 3: AST-grep Rules

Run `mise run ast-grep` and fix rule violations:

Reference: `project-conventions/references/ast-grep-rules.md`

| Rule                       | Fix                                  |
| -------------------------- | ------------------------------------ |
| `error-context-required`   | Add `.context("msg")?`               |
| `no-blocking-in-async`     | Use tokio equivalents                |
| `no-get-prefix`            | Remove `get_` prefix                 |
| `no-hardcoded-credentials` | Load from env/config                 |
| `secure-random-required`   | Use `OsRng`/`ChaCha20Rng`            |
| `module-size-limit`        | Split module (< 500 lines, < 10 fns) |

Suppression (last resort): `// ast-grep-ignore: <rule-id>`

### Step 4: Pre-commit Final Check

Run `mise run pre-commit` (runs `fmt:check` + `clippy:strict` + `ast-grep`).

- **All pass** → Stage relevant files. Recommend commit.
- **Errors remain** → Read `coding/references/common-errors.md`, fix, return to Step 1.

### Step 5: Test Verification

Run `mise run test` to confirm no regressions.

## Subagent Templates

Use Task tool with `subagent_type: "general-purpose"` for parallel fixes:

| Template                | Purpose                 |
| ----------------------- | ----------------------- |
| `prompts/fix-lint.md`   | Fix ast-grep violations |
| `prompts/fix-clippy.md` | Fix clippy warnings     |

## Workflow Position

**Cycle**: `/coding` → `/qa` → `/review code` → `/docs` → `/review docs`
This agent follows `/coding`. After all checks pass, run `/review code`.
