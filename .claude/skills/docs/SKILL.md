---
name: docs
description: >-
  Documentation agent for generating and updating spec documents, rustdoc
  comments, and roadmap files. Use /docs to create specs, add rustdoc to
  public APIs, or update PLAN.md / IMPROVEMENT_PLAN.md.
---

# Docs Agent — Spec / Rustdoc / Roadmap

## Document Type Detection

Determine the type based on user request or context:

| Request                          | Type    | Template                              |
| -------------------------------- | ------- | ------------------------------------- |
| "spec を作成" / new spec         | spec    | `references/spec-templates.md`        |
| "rustdoc を追加" / add docs      | rustdoc | `references/rustdoc-patterns.md`      |
| "ロードマップ更新" / update plan | roadmap | Follow existing format in target file |

---

## Spec Documents

Reference: `references/spec-templates.md`

### Workflow

1. Read existing specs in `docs/specs/` for style consistency.
2. Determine placement:
   - Component spec → `docs/specs/components/<name>.md`
   - External API spec → `docs/specs/external/<service>/<name>.md`
   - Research → `docs/specs/api-research/<name>.md`
3. Write spec following template:
   - Parent document link (required)
   - Background section with problem statement
   - Mermaid flow diagram (required)
   - Table specifications for data/API
   - Considerations with checkboxes
4. Language: **Japanese** for spec content.
5. Formatting rules:
   - Half-width brackets `()`
   - Alphanumeric in backticks with spaces: `` `value` ``
   - Code blocks in English

---

## Rustdoc Comments

Reference: `references/rustdoc-patterns.md`

### Workflow

1. Identify all `pub` items in the target module.
2. Add documentation following patterns:
   - Functions: 1-line summary + `# Errors` (if `Result`)
   - Types/Structs: concise description of purpose
   - Modules: `//!` doc comment (2 lines max)
   - Traits: purpose + intended usage
3. Language: **English** for all code documentation.
4. Run `mise run test:doc` to verify doc tests compile.

---

## Roadmap Updates

### Workflow

1. Read the target file (`docs/specs/PLAN.md` or `docs/specs/IMPROVEMENT_PLAN.md`).
2. Follow the existing format exactly.
3. Update status, add new items, or modify dependency graph.
4. Language: **Japanese** for roadmap content.

---

## Subagent Templates

Use Task tool for parallel documentation work:

| Template                      | Agent type        | Purpose                             |
| ----------------------------- | ----------------- | ----------------------------------- |
| `prompts/generate-rustdoc.md` | `general-purpose` | Add rustdoc to a module's pub items |
| `prompts/generate-spec.md`    | `general-purpose` | Generate a spec from requirements   |

## Workflow Position

**Cycle**: `/coding` → `/qa` → `/review code` → `/docs` → `/review docs`
This agent follows code review. After documentation is complete, run `/review docs`.
