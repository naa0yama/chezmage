# ast-grep Custom Rules

Source: `/app/ast-rules/*.yml`
Run: `mise run ast-grep`

## Suppression

- Suppress next line: `// ast-grep-ignore`
- Suppress specific rule: `// ast-grep-ignore: <rule-id>`
- All rules automatically exclude `#[cfg(test)]` modules.

## Rules

### 1. `error-context-required` (warning)

**Triggers**: `$EXPR?` without `.context()` or `.with_context()` in non-test code.
Supports multiline method chains (AST-based matching).

**Fix**: Add `.with_context(|| format!("failed to {operation}: {arg}"))?;` with
the operation description and argument value.

### 2. `no-blocking-in-async` (error)

**Triggers**: `std::fs`, `std::thread::sleep`, `std::net`, `std::process::Command`
inside `async fn`.

**Fix**: Replace with tokio equivalents (`tokio::fs`, `tokio::time::sleep`, etc.).

### 3. `no-get-prefix` (warning)

**Triggers**: Methods named `fn get_*()` inside `impl` blocks.

**Fix**: Remove the `get_` prefix. Use `fn name(&self)` instead of `fn get_name(&self)`.
Follows Rust API guidelines (C-GETTER).

### 4. `no-hardcoded-credentials` (error)

**Triggers**:

- Variables named `password`, `secret`, `key`, `token`, `credential` assigned
  a string literal of 8+ characters.
- API-key patterns (`sk-...`, `pk-...`, `AKIA...`, JWT format, 32+ char hex/alphanumeric).

**Exceptions**: Strings containing `test`, `demo`, `example`, `sample`.

**Fix**: Load from environment variables or config files.

### 5. `secure-random-required` (warning)

**Triggers**: `thread_rng()`, `rand::thread_rng()`, or `fastrand` used in
security contexts (function/variable names containing `security`, `crypto`,
`auth`, `token`, `key`, `password`, `salt`, `nonce`, `iv`).

**Fix**: Use `OsRng` or `ChaCha20Rng` (types implementing `CryptoRng`).

### 6. `module-size-limit` (warning)

**Triggers**: Modules with 10+ function definitions.

**Fix**: Split into smaller focused modules. Target < 500 lines / < 10 functions per module.
