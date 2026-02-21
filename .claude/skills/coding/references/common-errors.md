# Common Pre-commit Errors

Frequent errors encountered during `mise run pre-commit` and their fixes.
Referenced by both `/coding` and `/qa` agents.

## 1. `error-context-required` (ast-grep)

**Symptom**: `Bare ? without .context() or .with_context()`

**Fix**: Add context to every `?` operator.

```rust
// Before
let val = some_call()?;

// After
let val = some_call().context("some_call failed")?;
// or
let val = some_call().with_context(|| format!("failed for {param}"))?;
```

## 2. `no-blocking-in-async` (ast-grep)

**Symptom**: Blocking call inside `async fn`

**Fix**: Replace with tokio equivalents.

| Blocking                | Async equivalent          |
| ----------------------- | ------------------------- |
| `std::fs::*`            | `tokio::fs::*`            |
| `std::thread::sleep`    | `tokio::time::sleep`      |
| `std::net::*`           | `tokio::net::*`           |
| `std::process::Command` | `tokio::process::Command` |

## 3. `no-get-prefix` (ast-grep)

**Symptom**: Method named `fn get_*()` in `impl` block

**Fix**: Remove the `get_` prefix. Use `fn name(&self)` not `fn get_name(&self)`.

## 4. `unwrap_used` / `expect_used` (clippy)

**Symptom**: `unwrap()` or `expect()` in non-test code

**Fix**: Replace with `.context("msg")?` or pattern matching.

Exception: `#![allow(clippy::unwrap_used)]` is permitted in `#[cfg(test)]` modules.

## 5. `print_stdout` / `print_stderr` / `dbg_macro` (clippy)

**Symptom**: `println!`, `eprintln!`, or `dbg!` in production code

**Fix**: Replace with `tracing` macros.

```rust
// Before
println!("fetched {} items", count);

// After
tracing::info!(count, "fetched items");
```

Exception: `build.rs` may use `println!` with `// ast-grep-ignore: no-println-debug`.

## 6. `fmt:check` failure

**Symptom**: Formatting does not match expected style

**Fix**: Run `mise run fmt` to auto-format, then re-stage files.

## 7. `module-size-limit` (ast-grep)

**Symptom**: Module has 10+ function definitions

**Fix**: Split into smaller focused modules. Target < 500 lines, < 10 functions.

## 8. Import ordering issues (clippy / fmt)

**Symptom**: Imports not properly grouped or ordered

**Fix**: Group imports with blank lines: `std` → external crates → `crate`/`super`.
