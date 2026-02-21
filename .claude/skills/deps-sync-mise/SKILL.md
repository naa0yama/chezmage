---
name: deps-sync-mise
description: >-
  Mise tool investigation logic. Resolves tool source via mise registry,
  then queries GitHub API and CHANGELOG to identify changes since Claude's
  knowledge cutoff. Falls back to WebSearch for documentation sites when
  CHANGELOG is unavailable. Reusable from other skills (called by
  deps-sync orchestrator).
---

# deps-sync-mise — Mise Tool Investigation

## Purpose

Investigate a single mise-managed tool for changes since Claude's knowledge
cutoff (May 2025). Produce a structured report that the `deps-sync`
orchestrator uses to generate a `tool-<name>/SKILL.md` skill file.

## Input

The caller provides:

| Field       | Example  | Description                            |
| ----------- | -------- | -------------------------------------- |
| `tool_name` | `zizmor` | Tool name as it appears in `mise.toml` |
| `version`   | `1.16.0` | Pinned version from `mise.toml`        |

## Investigation Flow

### Step 1: Determine registry type

Run via Bash:

```
mise registry <tool_name>
```

This returns the tool's source in one of these formats:

| Format                | Meaning                         | Action                               |
| --------------------- | ------------------------------- | ------------------------------------ |
| `aqua:<owner>/<repo>` | GitHub-hosted binary            | Proceed to Step 3                    |
| `cargo:<crate>`       | Rust crate installed via cargo  | Delegate to `deps-sync-crates` flow  |
| `core:<tool>`         | Built-in runtime (node, python) | **Skip** — not a project dependency  |
| `npm:<package>`       | npm package                     | Proceed to Step 3 (use npm registry) |

If the tool is `cargo:*`, hand off to the `deps-sync-crates` skill
with the crate name extracted from the registry output.

If the tool is `core:*`, **stop here** and report "core tool — skipped."

### Step 2: Get current installed version

Run via Bash:

```
mise exec -- <tool_name> --version
```

or for tools that use a different flag:

```
mise exec -- <tool_name> version
```

Record the actual installed version for comparison.

### Step 3: GitHub API — Get repository info (aqua registry)

Parse `<owner>/<repo>` from the aqua registry output.

```
WebFetch https://api.github.com/repos/<owner>/<repo>
```

Extract:

- `default_branch` — typically `main` or `master`
- `description` — tool description for the skill file

### Step 4: CHANGELOG — Get raw changelog text

Try these URLs in order (stop on first success):

1. `WebFetch https://raw.githubusercontent.com/<owner>/<repo>/<branch>/CHANGELOG.md`
2. `WebFetch https://raw.githubusercontent.com/<owner>/<repo>/<branch>/Changelog.md`
3. `WebFetch https://raw.githubusercontent.com/<owner>/<repo>/<branch>/CHANGES.md`

### Step 5: Fallback — Search for documentation site

**Only if all CHANGELOG URLs fail in Step 4.**

```
WebSearch "<tool_name> documentation site"
```

Look for official documentation with release notes or changelog sections.
Examples:

- zizmor → `docs.zizmor.sh/release-notes/`
- dprint → `dprint.dev/blog/`

Fetch the release notes page:

```
WebFetch <documentation_url>
```

### Step 6: Extract post-cutoff changes

From the CHANGELOG text (or documentation), extract entries dated after
May 2025. Categorize them:

- **CLI flag/option changes** — renamed, removed, or new flags
- **Config file changes** — new fields, format changes, deprecated options
- **New features/rules** — new capabilities, lint rules, commands
- **Deprecated features** — features scheduled for removal
- **Breaking changes** — anything requiring user action on upgrade

### Step 7: Compile report

Structure the findings as:

```
## Changes Since Knowledge Cutoff

### CLI Changes
- [flag/option changes with old → new examples]

### Config Changes
- [configuration file format changes]

### New Features
- [new capabilities with usage examples]

### Breaking Changes
- [changes requiring action]

### Deprecated Features
- [features being removed with migration path]

## Gotchas
- [common mistakes, edge cases]

## Project Impact
- **Config files**: [relevant config file paths in project]
- **Task usage**: [mise tasks that use this tool]
- **Action needed**: [none / update config / update flags]
```

## Rules

- **Registry-first**: Always start with `mise registry` to determine the tool source
- **Delegate cargo tools**: `cargo:*` tools use the `deps-sync-crates` flow
- **Skip core tools**: `core:*` tools (node, python, etc.) are out of scope
- **Deterministic URLs preferred**: Use GitHub API and raw.githubusercontent.com first
- **WebSearch only as fallback**: Only use WebSearch when CHANGELOG cannot be found via deterministic URLs
- **Language**: All code examples and comments in English

## Output Format

Return a structured report following the template above. The `deps-sync`
orchestrator will use this report to generate the final `tool-<name>/SKILL.md`.
