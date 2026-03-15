# Module & Project Structure — chezmage

> **Shared patterns**: See `~/.claude/skills/rust-project-conventions/references/module-structure.md`
> for visibility rules, mod.rs re-export pattern, size limits, CLI design, and clippy configuration.

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

## OTel / Tracing Setup

- OTel is opt-in (`default = []`).
- Build with OTel: `mise run build -- --features otel`.
- Set `OTEL_EXPORTER_OTLP_ENDPOINT` env var to activate OTLP export.
- Without the env var (or empty), only the `fmt` layer is active.
- Test tasks automatically set `OTEL_EXPORTER_OTLP_ENDPOINT=""` to prevent OTel panics.
- Feature flag in `Cargo.toml`:
  ```toml
  [features]
  default = []
  otel = [
  	"dep:opentelemetry",
  	"dep:opentelemetry_sdk",
  	"dep:opentelemetry-otlp",
  	"dep:tracing-opentelemetry",
  ]
  ```
