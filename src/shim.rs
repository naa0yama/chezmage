//! Shim mode: pipe age key from env var to the real `age` binary via stdin.

use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::exec::{ENV_AGE_KEY, find_real_age, replace_process};

/// Run in shim mode: read the age key from `ENV_AGE_KEY` and pipe it to the
/// real `age` binary via stdin.
///
/// If `ENV_AGE_KEY` is not set or the args contain no `-i` / `--identity`
/// flags, falls back to executing the real `age` directly.
///
/// # Errors
///
/// Returns an error if the real `age` binary cannot be found or spawned.
pub fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    let age_key = env::var(ENV_AGE_KEY).ok().filter(|k| !k.is_empty());

    let Some(age_key) = age_key else {
        let age = find_real_age()?;
        let err = replace_process(&age, &args);
        bail!("failed to exec age: {err}");
    };

    let (has_identity, new_args) = rewrite_identity_args(&args);

    if !has_identity {
        let age = find_real_age()?;
        let err = replace_process(&age, &args);
        bail!("failed to exec age: {err}");
    }

    let age = find_real_age()?;
    let mut child = Command::new(&age)
        .args(&new_args)
        .stdin(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn age: {}", age.display()))?;

    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "{age_key}").context("failed to write key material to age stdin")?;
        drop(stdin);
    }

    let status = child.wait().context("waiting for age process")?;

    #[allow(clippy::exit)]
    std::process::exit(status.code().unwrap_or(1));
}

/// Rewrite `-i <path>` / `--identity <path>` / `--identity=<path>` args
/// into a single `-i -` for stdin-based key delivery.
///
/// Returns `(has_identity, rewritten_args)`.
#[must_use]
pub fn rewrite_identity_args(args: &[String]) -> (bool, Vec<String>) {
    let mut has_identity = false;
    let mut new_args = Vec::with_capacity(args.len());
    let mut skip_next = false;
    let mut inserted = false;

    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }

        if arg == "-i" || arg == "--identity" {
            has_identity = true;
            skip_next = true;
            if !inserted {
                new_args.push(String::from("-i"));
                new_args.push(String::from("-"));
                inserted = true;
            }
        } else if arg.starts_with("-i=") || arg.starts_with("--identity=") {
            has_identity = true;
            if !inserted {
                new_args.push(String::from("-i"));
                new_args.push(String::from("-"));
                inserted = true;
            }
        } else {
            new_args.push(arg.clone());
        }
    }

    (has_identity, new_args)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn s(val: &str) -> String {
        String::from(val)
    }

    #[test]
    fn test_rewrite_short_identity() {
        // Arrange
        let args = vec![s("-d"), s("-i"), s("/path/key.txt"), s("file.age")];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args);

        // Assert
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("-i"), s("-"), s("file.age")]);
    }

    #[test]
    fn test_rewrite_long_identity() {
        // Arrange
        let args = vec![s("-d"), s("--identity"), s("/path/key.txt"), s("file.age")];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args);

        // Assert
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("-i"), s("-"), s("file.age")]);
    }

    #[test]
    fn test_rewrite_identity_equals() {
        // Arrange
        let args = vec![s("-d"), s("--identity=/path/key.txt"), s("file.age")];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args);

        // Assert
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("-i"), s("-"), s("file.age")]);
    }

    #[test]
    fn test_rewrite_short_equals() {
        // Arrange
        let args = vec![s("-d"), s("-i=/path/key.txt"), s("file.age")];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args);

        // Assert
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("-i"), s("-"), s("file.age")]);
    }

    #[test]
    fn test_rewrite_multiple_identities() {
        // Arrange
        let args = vec![
            s("-d"),
            s("-i"),
            s("/path/a.gpg"),
            s("-i"),
            s("/path/b.gpg"),
            s("file.age"),
        ];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args);

        // Assert
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("-i"), s("-"), s("file.age")]);
    }

    #[test]
    fn test_rewrite_no_identity() {
        // Arrange
        let args = vec![s("-e"), s("-r"), s("age1xxx"), s("file.txt")];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args);

        // Assert
        assert!(!has_id);
        assert_eq!(new_args, args);
    }

    #[test]
    fn test_rewrite_mixed_formats() {
        // Arrange
        let args = vec![
            s("-d"),
            s("-i"),
            s("/path/a.gpg"),
            s("--identity=/path/b.gpg"),
            s("file.age"),
        ];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args);

        // Assert
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("-i"), s("-"), s("file.age")]);
    }

    #[test]
    fn test_rewrite_empty_args() {
        // Arrange
        let args: Vec<String> = Vec::new();

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args);

        // Assert
        assert!(!has_id);
        assert!(new_args.is_empty());
    }

    #[test]
    fn test_rewrite_identity_at_end_without_value() {
        // Arrange: -i at end with no following value
        let args = vec![s("-d"), s("file.age"), s("-i")];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args);

        // Assert: -i detected even without value; skip_next consumes nothing
        assert!(has_id);
        assert_eq!(new_args, vec![s("-d"), s("file.age"), s("-i"), s("-")]);
    }

    #[test]
    fn test_rewrite_preserves_other_flags_order() {
        // Arrange: non-identity flags should be preserved in order
        let args = vec![
            s("-d"),
            s("-o"),
            s("output.txt"),
            s("-i"),
            s("/path/key.gpg"),
            s("input.age"),
        ];

        // Act
        let (has_id, new_args) = rewrite_identity_args(&args);

        // Assert
        assert!(has_id);
        assert_eq!(
            new_args,
            vec![
                s("-d"),
                s("-o"),
                s("output.txt"),
                s("-i"),
                s("-"),
                s("input.age")
            ]
        );
    }
}
