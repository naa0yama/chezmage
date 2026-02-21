# Module & Project Structure

## Visibility

- Default to private. Expose only what is needed.
- Prefer `pub(crate)` over `pub` for internal APIs.
- Use selective `pub use` re-exports in `mod.rs`.

## mod.rs Re-export Pattern

```rust
//! Module-level doc comment describing the module purpose.

mod internal_a;
mod internal_b;
pub(crate) mod shared_internal;

#[allow(clippy::module_name_repetitions)]
pub use internal_a::TypeA;
pub use internal_b::{function_b, TypeB};
```

Key points:

- Add `//!` doc comments at the top of `mod.rs`.
- Use `#[allow(clippy::module_name_repetitions)]` when the re-exported type
  name contains the module name (e.g., `SyoboiClient` from `syoboi` module).
- Keep sub-modules private; expose types via `pub use`.

## Size Limits

- Maximum ~500 lines per module.
- Maximum ~10 functions per module (enforced by ast-grep `module-size-limit`).
- Split large modules into focused sub-modules.

## Project Source Layout

```
src/
  main.rs              # CLI entry point (clap derive)
  libs.rs              # Top-level library module
  libs/
    syoboi/            # Feature module (API client)
      mod.rs           # Re-exports
      api.rs           # API trait + implementation
      client.rs        # HTTP client + builder
      params.rs        # Query parameters
      rate_limiter.rs  # Rate limiting logic
      types.rs         # Data structures
      util.rs          # Utility functions
      xml.rs           # XML parsing
tests/
  cli_api_test.rs      # Integration tests (assert_cmd)
fixtures/
  syoboi/              # Test fixtures (XML)
ast-rules/
  *.yml                # Custom ast-grep lint rules
```

## CLI Design

Uses clap derive API with nested subcommands:

```rust
#[derive(Parser)]
#[command(about, version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
```

- All structs/enums get `///` doc comments (shown in `--help`).
- Use `#[arg(long)]` for named arguments; `#[arg(value_delimiter = ',')]`
  for comma-separated lists.
- Runtime: `#[tokio::main(flavor = "current_thread")]`.
- Git hash in version via `build.rs`.

## OTel / Tracing Setup

- Default build: `tracing-subscriber` with `fmt` layer only.
- OTel build: `cargo build --features otel` (via `mise run build -- --features otel`).
- Set `OTEL_EXPORTER_OTLP_ENDPOINT` env var to enable OTLP export.
- Feature flag in `Cargo.toml`:
  ```toml
  [features]
  otel = [
  	"dep:opentelemetry",
  	"dep:opentelemetry_sdk",
  	"dep:opentelemetry-otlp",
  	"dep:tracing-opentelemetry",
  ]
  ```

## Clippy Configuration

Strict lint profile defined in `Cargo.toml` under `[lints.clippy]`:

- Base: `all`, `pedantic`, `nursery`, `cargo` at warn level.
- Safety: `unwrap_used`, `expect_used`, `panic`, `indexing_slicing` warned.
- Security: `print_stdout`, `print_stderr`, `dbg_macro` warned.
- Type safety: `as_conversions`, `cast_possible_truncation` warned.
- Exceptions: `multiple-crate-versions` and `cargo-common-metadata` allowed.

See `Cargo.toml` `[lints.clippy]` section for the full list (~55 rules).
