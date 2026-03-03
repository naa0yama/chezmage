# Testing Patterns

## Unit Test Template

```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn test_descriptive_name() {
        // Arrange
        let input = "value";

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

- `#![allow(clippy::unwrap_used)]` permitted in test modules only.
- Use Arrange / Act / Assert comments in each test.
- `use super::*` is the only allowed wildcard import.

## Integration Test Template

File: `tests/<name>.rs`

```rust
#![allow(clippy::unwrap_used)]
#![allow(missing_docs)]

use assert_cmd::cargo_bin_cmd;
use predicates::prelude::predicate;
use tempfile::TempDir;

#[test]
#[cfg_attr(miri, ignore)]
fn test_cli_subcommand() {
    // Arrange & Act & Assert
    let mut cmd = cargo_bin_cmd!("chezmage");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("chezmage"));
}
```

- Use `assert_cmd::cargo_bin_cmd!` macro, chain `.assert().success()` / `.failure()`.
- Use `predicates::str::contains()` for output content checks.
- Add `#[cfg_attr(miri, ignore)]` — process-spawning tests cannot run under Miri.

## Tempfile Pattern

Use `tempfile::TempDir` for tests requiring file system access:

```rust
#[test]
#[cfg_attr(miri, ignore)] // tempfile I/O unsupported under Miri isolation
fn test_with_temp_files() {
    // Arrange
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("chezmoi.toml");
    std::fs::write(&config_path, "[age]\nidentity = \"/tmp/key.txt\"").unwrap();

    // Act & Assert
    // ...
}
```

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

## Coverage

Target: 80%+ line coverage. Run: `mise run coverage`
