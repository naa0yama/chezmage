---
name: review
description: >-
  Three-mode review agent for code, design, and documentation. Runs systematic
  checklists against changes. Use /review code, /review design, or /review docs
  to start a targeted review. Auto-detects mode from changed files if no
  argument is given.
---

# Review Agent — Code / Design / Docs

## Mode Selection

### Explicit mode

- `/review code` — review Rust source code changes
- `/review design` — review spec/design documents
- `/review docs` — review documentation (Japanese conventions, links, diagrams)

### Auto-detection (no argument)

Check changed files with `git diff --name-only`:

| File pattern      | Mode                            |
| ----------------- | ------------------------------- |
| `*.rs`            | code                            |
| `docs/specs/**`   | design                          |
| `docs/**`, `*.md` | docs                            |
| Mixed             | Run multiple modes sequentially |

---

## Code Review Mode

### Prerequisites

Read:

- `/app/.claude/skills/project-conventions/SKILL.md` — all project rules
- `/app/.claude/skills/project-conventions/references/ast-grep-rules.md` — custom lint rules
- `/app/.claude/skills/review/references/code-checklist.md` — 5-phase checklist

### Workflow

1. Run `git diff` to identify changed files and lines.
2. Walk through the 5 phases in `references/code-checklist.md`:
   - Phase 1: Automated checks pass
   - Phase 2: Error handling completeness
   - Phase 3: Test quality
   - Phase 4: API design
   - Phase 5: Security
3. Output findings in format: `file:line: [severity]: description`
   - Severity: `error` | `warning` | `info`
4. Summarize: total findings by severity, pass/fail recommendation.

---

## Design Review Mode

### Prerequisites

Read:

- `/app/.claude/skills/review/references/design-checklist.md` — spec requirements
- `/app/.claude/skills/project-conventions/references/module-and-project-structure.md` — architecture

### Workflow

1. Identify the spec file under review.
2. Check required sections per `references/design-checklist.md`.
3. Cross-reference with existing specs:
   - `docs/specs/PLAN.md` — overall plan
   - `docs/specs/IMPROVEMENT_PLAN.md` — dependency graph
4. Verify architecture alignment with project structure.
5. Output findings with section references.

---

## Docs Review Mode

### Workflow

1. Identify documentation files to review.
2. Check Japanese language conventions (from `CLAUDE.md`):
   - Half-width brackets (not full-width)
   - Half-width alphanumeric in backticks with spaces: `` `value` ``
   - Code comments in English, specs in Japanese
3. Check mermaid diagram syntax (if present):
   - Valid `flowchart TD` / `sequenceDiagram` structure
   - No broken references between nodes
4. Check internal links:
   - All `[text](path)` links point to existing files
   - Parent document links are correct
5. Output findings with file and line references.

---

## Subagent Templates

Use Task tool for parallel reviews:

| Template                   | Agent type        | Purpose                                 |
| -------------------------- | ----------------- | --------------------------------------- |
| `prompts/review-code.md`   | `general-purpose` | Review code files against checklist     |
| `prompts/review-design.md` | `Explore`         | Check spec completeness and consistency |
| `prompts/review-docs.md`   | `general-purpose` | Check docs conventions and links        |

## Workflow Position

**Cycle**: `/coding` → `/qa` → `/review code` → `/docs` → `/review docs`
Code review follows `/qa`. After review, run `/docs` for documentation updates.
