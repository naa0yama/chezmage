# Rustdoc Patterns

Patterns for writing documentation comments extracted from project code.

## Function Documentation

```rust
/// Brief one-line description of what the function does.
///
/// Extended description if needed (optional).
///
/// # Errors
///
/// Returns an error if {condition}.
pub fn function_name(param: Type) -> Result<ReturnType> {
```

For simple functions, the one-line summary is sufficient:

```rust
/// Extracts episode number and subtitle pairs from raw `SubTitles` text.
#[must_use]
pub fn parse_sub_titles(raw: &str) -> Vec<(u32, String)> {
```

## Async Function Documentation

```rust
/// Fetches all programs in the given time range, automatically paginating
/// when the API returns the maximum of 5,000 items per request.
///
/// # Errors
///
/// Returns an error if `params.range` is `None`, any underlying API request
/// fails, or timestamp conversion fails.
pub async fn lookup_all_programs(
```

## Type Documentation

```rust
/// HTTP client for Syoboi Calendar API with rate limiting.
#[derive(Debug)]
pub struct SyoboiClient {
```

## Enum Documentation

```rust
/// Query parameters for the `ProgLookup` API command.
#[derive(Debug, Clone, Default)]
pub struct ProgLookupParams {
    /// Time range filter for program lookup.
    pub range: Option<TimeRange>,
    /// Channel IDs to filter by.
    pub ch_ids: Option<Vec<u32>>,
}
```

## Module Documentation

```rust
//! Syoboi Calendar API client and data types.

mod internal;
pub use internal::PublicType;
```

Keep module doc comments to 1-2 lines. Place at the very top of the file.

## Trait Documentation

```rust
/// Local abstraction for Syoboi Calendar API operations.
///
/// Implementations handle HTTP communication and XML parsing.
pub trait LocalSyoboiApi {
    /// Looks up title information by TID.
    async fn lookup_titles(&self, tids: &[u32]) -> Result<Vec<SyoboiTitle>>;
}
```

## Re-export Documentation

```rust
//! Syoboi Calendar API module.

mod api;
mod client;

#[allow(clippy::module_name_repetitions)]
pub use client::SyoboiClient;
pub use api::LocalSyoboiApi;
```

## Rules

1. All doc comments in **English**.
2. Every `pub fn` returning `Result` must have `# Errors`.
3. One-line summary is mandatory; extended description is optional.
4. Use `#[must_use]` on pure functions returning values.
5. Module `//!` comments required in `mod.rs` files.
