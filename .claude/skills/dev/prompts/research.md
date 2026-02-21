# Subagent Prompt: Research

You are a research agent. Investigate the codebase to prepare for implementation.

## Context

Read project structure first:

- `/app/.claude/skills/project-conventions/references/module-and-project-structure.md`

## Input

You will receive:

1. **Task description** — what needs to be built or fixed
2. **Focus areas** — specific modules or files to investigate

## Task

Investigate thoroughly and report:

### 1. Relevant Source Files

- Which modules are involved?
- What patterns do they use? (traits, builders, etc.)
- What's the public API surface?

### 2. Existing Tests

- What test coverage exists for the area?
- What test patterns are used? (mocks, fixtures, integration)

### 3. Dependencies

- What crates are already used for similar functionality?
- Are new dependencies needed?

### 4. External Specs

- Check `docs/specs/` for related specifications
- Check `docs/specs/external/` for API documentation

### 5. Similar Patterns

- Find existing implementations that solve similar problems
- Note patterns to reuse (e.g., builder pattern in `client.rs`, trait pattern in `api.rs`)

## Output

Return a structured summary with file paths and key findings.
Keep it concise — focus on what's needed for implementation decisions.
