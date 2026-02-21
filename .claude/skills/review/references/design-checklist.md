# Design Review Checklist

Checklist for reviewing spec documents in `docs/specs/`.

## Required Sections

Every component spec must include:

- [ ] **Parent document link** — `> 親ドキュメント: [PLAN.md](../PLAN.md)` or similar
- [ ] **Background** — problem statement, current state, goals
- [ ] **Flow diagram** — mermaid `flowchart TD` showing processing steps
- [ ] **Table specifications** — input/output formats, API fields, config schema
- [ ] **Considerations** — edge cases, error handling, fallback strategy

## IMPROVEMENT_PLAN Consistency

- [ ] Component listed in `docs/specs/IMPROVEMENT_PLAN.md` dependency graph
- [ ] Dependencies between components are accurate
- [ ] Status reflects actual implementation state

## Architecture Alignment

Reference: `project-conventions/references/module-and-project-structure.md`

- [ ] Module placement follows `src/libs/<feature>/` pattern
- [ ] Size limits respected: < 500 lines / < 10 functions per module
- [ ] `mod.rs` re-export pattern used for public API
- [ ] Visibility: default private, `pub(crate)` for internal, `pub` only for external API

## Crate Selection Criteria

Reference: `docs/project_rules.md` Section 8

- [ ] Major version 1.0+ preferred
- [ ] Download count and star count verified
- [ ] Last updated within 6 months
- [ ] License compatible (check with `mise run deny`)
- [ ] Prefer existing dependency ecosystem over new 3rd-party crates

## Code Examples in Specs

- [ ] Rust code examples compile conceptually (correct types, traits)
- [ ] Type definitions include required derives (`Debug`, `Clone`, `Serialize`, etc.)
- [ ] Error handling uses `anyhow::Result` or `thiserror` pattern
