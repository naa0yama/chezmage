---
name: deps-sync
description: >-
  Scan Cargo.toml, Cargo.lock, mise.toml, and mise.local.toml to detect
  all project dependencies. Investigate each package for changes since
  Claude's knowledge cutoff and generate lib-*/tool-* skills automatically.
  Use /deps-sync to run.
---

# deps-sync — Dependency Skill Generator

## Purpose

Automatically detect all project dependencies, investigate changes since
Claude's knowledge cutoff (May 2025), and generate `.claude/skills/lib-*`
and `.claude/skills/tool-*` skill files so Claude always has up-to-date
knowledge of the libraries and tools in this project.

## Trigger

User runs `/deps-sync`.

## Workflow

### Step 1: Package Detection

#### 1a. Rust crates from Cargo.toml

Read `/app/Cargo.toml` and extract all entries from `[workspace.dependencies]`.
For each crate, record:

- **name** — crate name (e.g., `ratatui`)
- **version_spec** — version specifier (e.g., `"0.29"`)
- **features** — enabled features list (e.g., `["derive"]`)

Skip internal crates (those with `path = "crates/..."` entries).

#### 1b. Resolved versions from Cargo.lock

Read `/app/Cargo.lock` and match each crate from Step 1a to its resolved
version. Record the exact locked version (e.g., `0.29.0`).

#### 1c. Mise tools from mise.toml

Read `/app/mise.toml` `[tools]` section. For each tool, record:

- **name** — tool name (e.g., `zizmor`, `dprint`)
- **version** — pinned version (e.g., `1.16.0`)

Note: Tools with `cargo:` prefix (e.g., `"cargo:ast-grep"`) are Rust crates
installed via cargo. Treat them as Rust crates for investigation, but generate
`tool-*` skills (not `lib-*`) since they are CLI tools, not library dependencies.

#### 1d. Mise local tools from mise.local.toml

Read `/app/mise.local.toml` if it exists. Same extraction as Step 1c.

Skip `core:*` tools (e.g., `node`) — these are runtimes, not project tools.
Skip `npm:*` tools with version `latest` — no stable version to track.

#### 1e. Check existing skills

List existing skills:

```
Glob .claude/skills/lib-*/SKILL.md
Glob .claude/skills/tool-*/SKILL.md
```

For each existing skill, read the `Version` line to determine if an update
is needed. If the resolved version matches the existing skill version,
mark that package as "up to date" and skip it.

### Step 2: Investigate Rust Crates

For each Rust crate identified in Step 1a (excluding up-to-date ones),
follow the investigation flow defined in:

```
/app/.claude/skills/deps-sync-crates/SKILL.md
```

**Parallelization**: Use Task tool with `subagent_type: "general-purpose"`
to investigate multiple crates concurrently. Each subagent should:

1. Read `/app/.claude/skills/deps-sync-crates/SKILL.md` for the procedure
2. Investigate the assigned crate
3. Return the structured report

Group closely related crates for a single investigation:

| Group         | Crates                                                                              |
| ------------- | ----------------------------------------------------------------------------------- |
| opentelemetry | `opentelemetry`, `opentelemetry-otlp`, `opentelemetry_sdk`, `tracing-opentelemetry` |
| tracing       | `tracing`, `tracing-subscriber`                                                     |

All other crates are investigated individually.

### Step 3: Investigate Mise Tools

For each mise tool identified in Steps 1c/1d (excluding up-to-date and
skipped ones), follow the investigation flow defined in:

```
/app/.claude/skills/deps-sync-mise/SKILL.md
```

**Parallelization**: Same as Step 2 — use Task subagents for concurrent
investigation.

`cargo:*` tools: Extract the crate name and use `deps-sync-crates` flow,
but generate `tool-*` skills since they are CLI tools.

### Step 4: Generate Skills

For each package with post-cutoff changes, generate a skill file.

#### Naming Convention

| Source                         | Prefix              | Example                        |
| ------------------------------ | ------------------- | ------------------------------ |
| Workspace dependency (library) | `lib-`              | `lib-ratatui`, `lib-quick-xml` |
| Mise tool / cargo tool         | `tool-`             | `tool-zizmor`, `tool-dprint`   |
| Grouped crates                 | `lib-` (group name) | `lib-opentelemetry`            |

- Replace underscores with hyphens in crate names (e.g., `quick_xml` → `lib-quick-xml`)
- Use the group name for grouped crates (e.g., `lib-opentelemetry` not `lib-opentelemetry-otlp`)

#### Skill File Template — Library (`lib-*`)

Write to `.claude/skills/lib-<name>/SKILL.md`:

```markdown
---
name: lib-<crate-name>
description: >-
  <crate description from crates.io>. Version <version> changes and usage
  notes. Use when writing code that uses <crate-name>.
---

# <crate-name> <latest_version>

## Project Usage

- **Version**: <resolved_version> (spec: <version_spec>)
- **Features**: <enabled features, or "default" if none>
- **Key files**: <list of files importing this crate>

## Changes Since Knowledge Cutoff

### Breaking Changes

<breaking changes with old → new code examples, or "None">

### New APIs

<new APIs relevant to this project, with code examples, or "None">

### Deprecations

<deprecated items with migration paths, or "None">

## Gotchas

<version conflicts, edge cases, common mistakes, or "None known">
```

#### Skill File Template — Tool (`tool-*`)

Write to `.claude/skills/tool-<name>/SKILL.md`:

```markdown
---
name: tool-<tool-name>
description: >-
  <tool description>. Version <version> changes and configuration notes.
  Use when running or configuring <tool-name>.
---

# <tool-name> <version>

## Project Usage

- **Version**: <installed_version>
- **Config**: <config file path if any>
- **Tasks**: <mise tasks that use this tool>

## Changes Since Knowledge Cutoff

### CLI Changes

<flag/option changes, or "None">

### Config Changes

<config format changes, or "None">

### New Features

<new capabilities, or "None">

### Breaking Changes

<changes requiring action, or "None">

## Gotchas

<common mistakes, edge cases, or "None known">
```

### Step 5: Skip Unchanged Packages

For packages with **no post-cutoff changes** (no releases after May 2025):

- Do **not** generate a skill file
- Log: `<package> — no changes since cutoff, skipping`

For packages that are **already up to date** (existing skill matches version):

- Do **not** regenerate the skill file
- Log: `<package> — already up to date (<version>), skipping`

### Step 6: Summary Report

After all investigations complete, output a summary table:

```
## deps-sync Results

| Package | Type | Status | Skill |
|---------|------|--------|-------|
| ratatui | crate | Updated (0.29.0 → 0.30.0) | lib-ratatui |
| quick-xml | crate | New skill | lib-quick-xml |
| zizmor | mise | No changes | — |
| tokio | crate | Skipped (no post-cutoff) | — |
| ... | ... | ... | ... |

Generated: X skills | Updated: Y skills | Skipped: Z packages
```

## Configuration

### Knowledge Cutoff Date

The cutoff date is **May 2025**. This is hardcoded based on Claude's
training data. When Claude's knowledge cutoff changes, update this date
in this file and in `deps-sync-crates/SKILL.md` and `deps-sync-mise/SKILL.md`.

### Excluded Packages

Exclusion is determined dynamically by inspecting `Cargo.toml` and `mise.toml`:

- **Path dependencies** — entries with `path = "..."` in `[workspace.dependencies]` (internal crates)
- **Dev-only crates** — entries listed under the `# Dev dependencies` comment block in `[workspace.dependencies]`
- **Core mise tools** — `core:*` entries detected via `mise registry` (runtimes like node, python)

To investigate dev dependencies, run with explicit override:
"Include dev dependencies in this deps-sync run."

## Subagent Strategy

For a typical run with ~20 packages:

1. **Batch 1** (parallel): 4-5 crate investigations via Task subagents
2. **Batch 2** (parallel): 4-5 more crate investigations
3. **Batch 3** (parallel): mise tool investigations
4. **Sequential**: Generate skill files from collected reports

Each subagent receives:

- The relevant `deps-sync-crates` or `deps-sync-mise` SKILL.md as context
- The specific package details (name, version, features)
- Instruction to return the structured report

## Workflow Position

This is a standalone utility skill. It does not participate in the
`/coding` → `/qa` → `/review` pipeline.

Run `/deps-sync` periodically (e.g., after `cargo update` or when starting
a new feature that uses an unfamiliar dependency).
