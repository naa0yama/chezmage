# Dev Workflow Reference

## Phase Decision Tree

```
Task received
  │
  ├─ Is it a typo/config fix?
  │   └─ Yes → Phase 3 (Code) → Phase 4 (QA) → Done
  │
  ├─ Is it docs-only?
  │   └─ Yes → Phase 5 (Docs) → Phase 6 (Review) → Done
  │
  ├─ Is it a bug fix?
  │   └─ Yes → Phase 1 (Research) → Phase 3 (Code) → Phase 4 (QA) → Done
  │
  ├─ Is it a refactor?
  │   └─ Yes → Phase 1 (Research) → Phase 3 (Code) → Phase 4 (QA) → Phase 6 (Review) → Done
  │
  └─ Is it a new feature?
      └─ Yes → All phases: 1 → 2 → 3 → 4 → 5 → 6 → Done
```

## Subagent Dispatch Map

| Phase          | Subagent type     | Prompt template                    |
| -------------- | ----------------- | ---------------------------------- |
| Research       | `Explore`         | `dev/prompts/research.md`          |
| Design         | `Plan`            | `dev/prompts/design.md`            |
| Code: tests    | `general-purpose` | `coding/prompts/write-tests.md`    |
| Code: impl     | `general-purpose` | `coding/prompts/implement.md`      |
| QA: clippy     | `general-purpose` | `qa/prompts/fix-clippy.md`         |
| QA: lint       | `general-purpose` | `qa/prompts/fix-lint.md`           |
| Docs: rustdoc  | `general-purpose` | `docs/prompts/generate-rustdoc.md` |
| Docs: spec     | `general-purpose` | `docs/prompts/generate-spec.md`    |
| Review: code   | `general-purpose` | `review/prompts/review-code.md`    |
| Review: design | `Explore`         | `review/prompts/review-design.md`  |
| Review: docs   | `general-purpose` | `review/prompts/review-docs.md`    |

## Escalation Thresholds

- **Phase 3 loop**: Max 5 RED→GREEN→REFACTOR cycles before escalating
- **Phase 4 loop**: Max 3 fmt→clippy→ast-grep→pre-commit cycles before escalating
- **Phase 6 findings**: Never escalate review findings — fix them internally

## Parallel Execution Opportunities

These phases can dispatch multiple subagents in parallel:

- **Phase 1**: Research source files + research specs + research tests simultaneously
- **Phase 3**: Write tests for independent modules simultaneously
- **Phase 4**: Fix clippy warnings + fix ast-grep violations simultaneously
- **Phase 5**: Generate rustdoc + update spec simultaneously
