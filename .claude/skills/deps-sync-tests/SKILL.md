---
name: deps-sync-tests
description: >-
  Scan project test files and dependencies to generate project-specific Miri
  compatibility categories. Called by the deps-sync orchestrator to update
  testing-patterns.md automatically.
---

# deps-sync-tests — Test Pattern Investigator

## Trigger

Called by the `deps-sync` orchestrator (Step 4: Investigate Test Patterns).
Not intended for standalone use.

## Investigation Flow

### Step 1: Classify Dependency Miri Impact

Read `/app/Cargo.toml` `[dependencies]` and `[dev-dependencies]` and classify
each package into Miri impact categories:

| Category              | Detection Method                                        | Examples (this project) |
| --------------------- | ------------------------------------------------------- | ----------------------- |
| File system           | Uses `tempfile` or heavy file I/O in tests              | tempfile                |
| Process spawning      | Tests use `std::process::Command` or `assert_cmd`       | assert_cmd              |
| Environment variables | Tests call `std::env::set_var` or rely on `HOME`        | —                       |
| Platform FFI (libc)   | Uses `libc` for pipes, signals, or process hardening    | libc, windows-sys       |
| Zeroize (custom Drop) | Uses `zeroize` derive which generates custom Drop impls | zeroize                 |

### Step 2: Scan Test Files for Miri Annotations

```
Grep pattern="#\[cfg_attr\(miri,\s*ignore\)\]" --type=rust
```

For each miri-ignored test, record:

- **File**: determined from file path
- **Function name**: the `fn` name on the line following the annotation
- **Reason**: extract from nearby comment; if absent, infer from test body
  (look for `tempfile`, `Command`, `assert_cmd`, `env::set_var`,
  `libc`, `pipe`, `HOME`, etc.)

### Step 3: Identify CI Crate-Level Exclusions

Read `.github/workflows/miri.yaml` (or `miri.yml`):

```
Grep pattern="--exclude" path=".github/workflows/" glob="miri.*"
```

Record each `--exclude <crate>` with its reason.

### Step 4: Count Statistics

Compute:

```
Total tests       = count of #[test] + #[tokio::test] in workspace
Miri-ignored      = count of #[cfg_attr(miri, ignore)] annotations
Crate-excluded    = count of tests in --exclude crates (from test count per crate)
Miri-compatible   = Total - Miri-ignored - Crate-excluded
```

Use Grep with `output_mode: "count"` for efficient counting:

```
Grep pattern="#\[(tokio::)?test\]" --type=rust output_mode="count"
Grep pattern="#\[cfg_attr\(miri,\s*ignore\)\]" --type=rust output_mode="count"
```

### Step 5: Generate Report

Produce the following markdown structure for insertion into
`testing-patterns.md`:

```markdown
## Miri Compatibility

For universal Miri rules and decision flowchart, see
`~/.claude/skills/rust-implementation/references/testing.md` → "Miri" section.

### Per-Test Skip Categories

1. **<Category> (<trigger crate>)** — <count> tests. <description>.
2. ...

### Statistics

| Metric                      | Count |
| --------------------------- | ----- |
| Total tests                 | X     |
| Miri-compatible             | Y     |
| Miri-ignored (per-test)     | Z     |
| Miri-excluded (crate-level) | W     |
```

## Output

Return the generated markdown section to the `deps-sync` orchestrator.
The orchestrator replaces the `## Miri Compatibility` section in:
`/app/.claude/skills/project-conventions/references/testing-patterns.md`
