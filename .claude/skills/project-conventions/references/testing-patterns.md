# Testing Patterns — Project-Specific

> **Shared templates**: See `~/.claude/skills/rust-coding/references/testing-templates.md`
> for unit test, async test, integration test templates, fixtures, coverage rules,
> and ETXTBSY workaround.

## Miri Compatibility

For universal Miri rules and decision flowchart, see
`~/.claude/skills/rust-implementation/references/testing.md` → "Miri" section.

### Per-Test Skip Categories

1. **File system (tempfile)** — 30 tests. Tests using `tempfile::tempdir()` or real file I/O. Miri has limited file system support under isolation mode.
2. **Process spawning (assert_cmd)** — 15 tests. Integration tests that execute the `chezmage` binary via `std::process::Command`. Miri cannot spawn external processes.
3. **Environment variables** — 5 tests. Tests relying on `HOME` env var or `std::env::set_var`. Env vars are not forwarded under Miri isolation.
4. **Platform FFI (libc / windows-sys)** — 4 tests. Tests using named pipes (`mkfifo`) or process hardening (`prctl`) via `libc` FFI. Miri cannot interpret foreign function calls.
5. **Zeroize (custom Drop)** — 9 tests. Tests for `SecureString` with `zeroize` derive. Custom `Drop` impls interact with Miri's stacked borrows model.

### Statistics

| Metric                      | Count |
| --------------------------- | ----- |
| Total tests                 | 141   |
| Miri-compatible             | 78    |
| Miri-ignored (per-test)     | 63    |
| Miri-excluded (crate-level) | 0     |
