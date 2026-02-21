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

## Async Test Template

```rust
#[tokio::test]
async fn test_async_operation() {
    // Arrange
    let mock = MockSyoboiApi::new(vec![batch1, batch2]);

    // Act
    let result = lookup_all_programs(&mock, &params).await.unwrap();

    // Assert
    assert_eq!(result.len(), expected_count);
}
```

## Mock Pattern

Implement traits on mock structs with pre-configured responses.
See `src/libs/syoboi/util.rs` tests for `MockSyoboiApi` example.

## Integration Test Template

File: `tests/<name>.rs`

```rust
#![allow(clippy::unwrap_used)]
#![allow(missing_docs)]

use assert_cmd::cargo_bin_cmd;
use predicates::prelude::predicate;

#[test]
fn test_cli_subcommand() {
    // Arrange & Act & Assert
    let mut cmd = cargo_bin_cmd!("dtvmgr");
    cmd.args(["api", "prog", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--time-since"));
}
```

- Use `assert_cmd::cargo_bin_cmd!` macro, chain `.assert().success()` / `.failure()`.
- Use `predicates::str::contains()` for output content checks.

## Fixtures & HTTP Mocking

Load fixtures with `include_str!`:

```rust
const FIXTURE: &str = include_str!("../../fixtures/syoboi/title_lookup_6309.xml");
```

Use `wiremock::MockServer` for HTTP mocking:

```rust
let mock_server = wiremock::MockServer::start().await;
wiremock::Mock::given(wiremock::matchers::method("GET"))
    .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(FIXTURE))
    .mount(&mock_server).await;
```

## Coverage

Target: 80%+ line coverage. Run: `mise run coverage`
