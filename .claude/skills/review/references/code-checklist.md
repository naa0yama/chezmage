# Code Review Checklist

5-phase systematic review based on `docs/project_rules.md` Section 15.

## Phase 1: Automated Checks

Verify all automated tools pass before manual review.

- [ ] `mise run fmt:check` — no formatting violations
- [ ] `mise run clippy:strict` — zero warnings
- [ ] `mise run ast-grep` — zero violations
- [ ] `mise run test` — all tests pass

If any fail, stop review and direct to `/qa` first.

## Phase 2: Error Handling

Every `?` must have context. No exceptions in production code.

- [ ] All `?` operators have `.context()` or `.with_context()`
- [ ] Error messages are descriptive (include variable values where useful)
- [ ] `anyhow::Result` for application code, `thiserror` for library errors
- [ ] No `unwrap()` / `expect()` in non-test code
- [ ] `#[allow(clippy::unwrap_used)]` only in `#[cfg(test)]` modules
- [ ] `# Errors` section in doc comments for fallible public functions

## Phase 3: Test Quality

Tests must follow Arrange / Act / Assert and cover edge cases.

- [ ] New/modified functions have corresponding tests
- [ ] Tests use Arrange / Act / Assert comments
- [ ] Unit tests in `#[cfg(test)] mod tests` with `use super::*`
- [ ] Async tests use `#[tokio::test]`
- [ ] Edge cases covered (empty input, boundary values, error paths)
- [ ] Mocks implement traits (see `MockSyoboiApi` pattern in `src/libs/syoboi/util.rs`)
- [ ] Fixtures loaded with `include_str!` from `fixtures/` directory
- [ ] Integration tests use `assert_cmd` + `predicates`
- [ ] Target: 80%+ line coverage (`mise run coverage`)

## Phase 4: API Design

Public interfaces must be minimal, well-named, and documented.

- [ ] Default to private; `pub(crate)` for internal APIs
- [ ] Selective `pub use` re-exports in `mod.rs`
- [ ] `//!` module doc comments in `mod.rs`
- [ ] No `get_` prefix on getters (Rust API guidelines C-GETTER)
- [ ] Conversion methods: `as_*` (cheap), `to_*` (expensive), `into_*` (consuming)
- [ ] `///` doc comments on all `pub` items
- [ ] Import grouping: `std` → external → `crate`/`super`
- [ ] No wildcard imports except `use super::*` in tests
- [ ] Module size: < 500 lines, < 10 functions

## Phase 5: Security & Async Safety

- [ ] No blocking I/O in `async fn` (use tokio equivalents)
- [ ] No hardcoded credentials (ast-grep `no-hardcoded-credentials`)
- [ ] `OsRng` for security-sensitive random (not `thread_rng`)
- [ ] No `unsafe` blocks (if present, must have `// SAFETY:` comment)
- [ ] External input validated at system boundaries
- [ ] `tracing` for all logging (no `println!` / `dbg!`)

## Output Format

```
src/libs/module.rs:42: [error]: bare ? without .context()
src/libs/module.rs:58: [warning]: missing test for error path
src/libs/module.rs:1: [info]: consider adding //! module doc comment
```

Severity guide:

- **error**: Must fix before merge (violations of mandatory rules)
- **warning**: Should fix (quality improvement)
- **info**: Optional improvement suggestion
