---
name: deps-sync-crates
description: >-
  Rust crate investigation logic. Queries crates.io API, GitHub API,
  CHANGELOG, and docs.rs to identify changes since Claude's knowledge
  cutoff. Uses only deterministic URLs — no WebSearch. Reusable from
  other skills (called by deps-sync orchestrator).
---

# deps-sync-crates — Rust Crate Investigation

## Purpose

Investigate a single Rust crate for changes since Claude's knowledge cutoff
(May 2025). Produce a structured report that the `deps-sync` orchestrator
uses to generate a `lib-<name>/SKILL.md` skill file.

## Input

The caller provides:

| Field              | Example      | Description                         |
| ------------------ | ------------ | ----------------------------------- |
| `crate_name`       | `ratatui`    | crates.io package name              |
| `version_spec`     | `0.29`       | Version specifier from `Cargo.toml` |
| `resolved_version` | `0.29.0`     | Locked version from `Cargo.lock`    |
| `features`         | `["derive"]` | Enabled features (may be empty)     |

## Investigation Flow

> **Rule**: Use only deterministic URLs. Never use `WebSearch` — it risks
> returning inaccurate information. Every URL must be constructable from
> known data.

### Step 1: crates.io API — Get crate metadata

```
WebFetch https://crates.io/api/v1/crates/<crate_name>
```

Extract from the JSON response:

- `crate.repository` — GitHub repository URL
- `crate.max_version` — latest published version
- `versions[]` — version list with `created_at` dates

Filter `versions[]` to find **post-cutoff releases** (created_at > 2025-05-01).
If no post-cutoff releases exist, **stop here** and report "no changes."

### Step 2: GitHub API — Get default branch

Parse `<owner>/<repo>` from the repository URL.

```
WebFetch https://api.github.com/repos/<owner>/<repo>
```

Extract `default_branch` (typically `main` or `master`).

### Step 3: CHANGELOG — Get raw changelog text

Try these URLs in order (stop on first success):

1. `WebFetch https://raw.githubusercontent.com/<owner>/<repo>/<branch>/CHANGELOG.md`
2. `WebFetch https://raw.githubusercontent.com/<owner>/<repo>/<branch>/Changelog.md`
3. `WebFetch https://raw.githubusercontent.com/<owner>/<repo>/<branch>/CHANGES.md`

If all fail, note "CHANGELOG not found" and proceed to Step 5.

### Step 4: Extract post-cutoff changes

From the CHANGELOG text, extract entries dated after May 2025.
Categorize them:

- **Breaking changes** — API removals, signature changes, behavioral changes
- **New APIs** — new functions, types, traits, modules
- **Deprecations** — deprecated items with migration paths
- **Bug fixes** — notable fixes that affect usage

### Step 5: docs.rs — Check API documentation

```
WebFetch https://docs.rs/<crate_name>/<latest_version>/
```

Look for:

- New public API items not in the CHANGELOG
- `#[deprecated]` annotations
- Feature flag changes

### Step 6: Project usage scan

Use `Grep` to find usage of this crate in the project:

```
Grep pattern: "use <crate_name>" (replace hyphens with underscores)
```

Also check `Cargo.toml` files for feature flags and version specs.
Record the key files that import this crate.

### Step 7: Compile report

Structure the findings as:

```
## Changes Since Knowledge Cutoff

### Breaking Changes
- [description with old → new pattern and code example]

### New APIs
- [relevant new APIs with brief code examples]

### Deprecations
- [deprecated items with migration path]

### Bug Fixes
- [notable fixes]

## Gotchas
- [version conflicts, edge cases, common mistakes]

## Project Impact
- **Key files**: [list of files using this crate]
- **Action needed**: [none / update code / review deprecations]
```

## Rules

- **Deterministic URLs only**: crates.io API → GitHub API → raw.githubusercontent.com
- **CHANGELOG filename fallback**: `CHANGELOG.md` → `Changelog.md` → `CHANGES.md`
- **Language**: All code examples and comments in English
- **Skip if no changes**: If no post-cutoff releases exist, return early with "no changes since cutoff"
- **Monorepo awareness**: Some crates live in monorepos (e.g., opentelemetry). The CHANGELOG may be at a different path — check for `<crate_name>/CHANGELOG.md` in the repo root if the top-level CHANGELOG doesn't mention the crate

## Output Format

Return a structured report following the template above. The `deps-sync`
orchestrator will use this report to generate the final `lib-<name>/SKILL.md`.
