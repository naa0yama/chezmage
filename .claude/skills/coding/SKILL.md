---
name: coding
description: >-
  TDD Red-Green-Refactor implementation cycle agent. Orchestrates test-first
  development using existing project-conventions and rust-implementation skills
  for coding rules. Use /coding to start a TDD workflow for new features or
  bug fixes.
---

# Coding Agent — TDD Red-Green-Refactor

## Prerequisites

Before starting, read:

- `/app/.claude/skills/project-conventions/SKILL.md` — project rules
- `/app/.claude/skills/project-conventions/references/testing-patterns.md` — test templates

## Workflow

### Step 0: Task Confirmation

- If a spec exists in `docs/specs/`, read it first.
- If a task list is active, check current task context.
- Identify the module and public API to implement.

### Step 1: RED — Write Failing Tests

Reference: `project-conventions/references/testing-patterns.md`

1. Create or open the test module (`#[cfg(test)] mod tests`).
2. Add `#![allow(clippy::unwrap_used)]` and `#![allow(clippy::indexing_slicing)]`.
3. Write tests following Arrange / Act / Assert pattern.
4. Choose test type:
   - **Unit**: `#[test]` with `use super::*`
   - **Async**: `#[tokio::test]` with mock structs
   - **Integration**: `tests/<name>.rs` with `assert_cmd`
5. Run `mise run test` — confirm tests **fail** (red).

### Step 2: GREEN — Minimal Implementation

Reference: `project-conventions/SKILL.md` (error context, imports, tracing)

1. Write the minimum code to make all tests pass.
2. Rules to follow:
   - Every `?` must have `.context()` or `.with_context()`
   - Use `tracing` macros, never `println!`
   - Imports grouped: `std` → external → `crate`/`super`
   - All commands via `mise run`, never `cargo` directly
3. Run `mise run test` — confirm tests **pass** (green).

### Step 3: REFACTOR — Improve Without Breaking

Reference: `~/.claude/skills/rust-implementation/references/naming.md`

1. Remove duplication, improve naming, simplify logic.
2. Follow Rust naming conventions (C-GETTER: no `get_` prefix, etc.).
3. Keep all tests green after each change.
4. Run `mise run test` after refactoring.

### Step 4: Repeat or Finish

- If more functionality is needed, return to Step 1.
- When complete, proceed to `/qa` for quality checks.

## Subagent Templates

Use Task tool with `subagent_type: "general-purpose"` for parallel work:

| Template                 | Purpose                                    |
| ------------------------ | ------------------------------------------ |
| `prompts/write-tests.md` | Generate tests for a target module         |
| `prompts/implement.md`   | Generate minimal implementation from tests |

## Reference Files

| File                          | Content                                                    |
| ----------------------------- | ---------------------------------------------------------- |
| `references/common-errors.md` | Frequent `pre-commit` errors and fixes (shared with `/qa`) |

## Workflow Position

**Cycle**: `/coding` → `/qa` → `/review code` → `/docs` → `/review docs`
This agent is the entry point. After implementation is complete, run `/qa`.
