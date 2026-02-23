//! GPG decryption utilities.

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

/// File extensions recognized as GPG-encrypted.
const GPG_EXTENSIONS: &[&str] = &[".gpg", ".asc"];

/// Validate and parse gpg output bytes into a trimmed string.
///
/// # Errors
///
/// Returns an error if the process exited non-zero or stdout is not valid UTF-8.
pub(crate) fn parse_gpg_bytes(
    success: bool,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    path: &Path,
) -> Result<String> {
    if !success {
        bail!(
            "gpg --decrypt {} failed (exit {})",
            path.display(),
            exit_code.unwrap_or(-1)
        );
    }

    let text = String::from_utf8(stdout).context("gpg output is not valid UTF-8")?;

    Ok(text.trim_end().to_owned())
}

/// Decrypt a GPG-encrypted file, returning its content as a string.
///
/// Inherits stdin so gpg-agent can interact with pinentry for PIN prompts.
/// GPG stderr is captured and logged (debug on success, warn on failure).
///
/// # Errors
///
/// Returns an error if `gpg` fails to execute, exits non-zero, or produces
/// invalid UTF-8 output.
pub fn decrypt(path: &Path) -> Result<String> {
    let output = Command::new("gpg")
        .args(["--quiet", "--yes", "--decrypt"])
        .arg(path)
        .stdin(Stdio::inherit())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to run gpg for {}", path.display()))?;

    if !output.stderr.is_empty() {
        let stderr_text = String::from_utf8_lossy(&output.stderr);
        if output.status.success() {
            tracing::debug!(path = %path.display(), stderr = %stderr_text.trim_end(), "gpg stderr");
        } else {
            tracing::warn!(path = %path.display(), stderr = %stderr_text.trim_end(), "gpg stderr");
        }
    }

    parse_gpg_bytes(
        output.status.success(),
        output.status.code(),
        output.stdout,
        path,
    )
}

/// Check whether a file path has a GPG encryption extension (.gpg or .asc).
#[must_use]
pub fn is_encrypted(path: &Path) -> bool {
    let name = path.to_string_lossy().to_lowercase();
    GPG_EXTENSIONS.iter().any(|ext| name.ends_with(ext))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::path::Path;

    use super::*;

    #[test]
    fn test_is_encrypted_gpg() {
        // Arrange & Act & Assert
        assert!(is_encrypted(Path::new("key.gpg")));
    }

    #[test]
    fn test_is_encrypted_asc() {
        // Arrange & Act & Assert
        assert!(is_encrypted(Path::new("key.asc")));
    }

    #[test]
    fn test_is_encrypted_uppercase() {
        // Arrange & Act & Assert
        assert!(is_encrypted(Path::new("key.GPG")));
    }

    #[test]
    fn test_is_encrypted_plaintext() {
        // Arrange & Act & Assert
        assert!(!is_encrypted(Path::new("key.txt")));
    }

    #[test]
    fn test_is_encrypted_no_extension() {
        // Arrange & Act & Assert
        assert!(!is_encrypted(Path::new("key")));
    }

    #[test]
    fn test_is_encrypted_with_path() {
        // Arrange & Act & Assert
        assert!(is_encrypted(Path::new(
            "/home/user/.config/chezmoi/age-key.gpg"
        )));
    }

    #[test]
    fn test_is_encrypted_mixed_case_asc() {
        // Arrange & Act & Assert
        assert!(is_encrypted(Path::new("key.AsC")));
    }

    #[test]
    fn test_parse_gpg_bytes_success() {
        // Arrange
        let stdout = b"decrypted content".to_vec();

        // Act
        let result = parse_gpg_bytes(true, Some(0), stdout, Path::new("key.gpg"));

        // Assert
        assert_eq!(result.unwrap(), "decrypted content");
    }

    #[test]
    fn test_parse_gpg_bytes_trims_trailing_newline() {
        // Arrange
        let stdout = b"secret key\n\n".to_vec();

        // Act
        let result = parse_gpg_bytes(true, Some(0), stdout, Path::new("key.gpg"));

        // Assert
        assert_eq!(result.unwrap(), "secret key");
    }

    #[test]
    fn test_parse_gpg_bytes_failure_exit_code() {
        // Arrange
        let stdout = Vec::new();

        // Act
        let result = parse_gpg_bytes(false, Some(2), stdout, Path::new("/tmp/key.gpg"));

        // Assert
        let err = result.unwrap_err().to_string();
        assert!(err.contains("/tmp/key.gpg"), "error should contain path");
        assert!(err.contains("exit 2"), "error should contain exit code");
    }

    #[test]
    fn test_parse_gpg_bytes_invalid_utf8() {
        // Arrange
        let stdout = vec![0xFF, 0xFE, 0x80];

        // Act
        let result = parse_gpg_bytes(true, Some(0), stdout, Path::new("key.gpg"));

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("UTF-8"));
    }

    #[test]
    fn test_parse_gpg_bytes_empty_stdout() {
        // Arrange
        let stdout = Vec::new();

        // Act
        let result = parse_gpg_bytes(true, Some(0), stdout, Path::new("key.gpg"));

        // Assert
        assert_eq!(result.unwrap(), "");
    }
}
