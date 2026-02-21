//! Wrapper mode: read chezmoi config, decrypt GPG identities, exec chezmoi.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::config::{dirs, filter_dummy_identities, find, read_identities};
use crate::exec::{ENV_AGE_KEY, ENV_GPG_KEY_FILE, expand_tilde, find_in_path, replace_process};
use crate::gpg::{decrypt, is_encrypted};
use crate::secure::SecureString;

/// Run in wrapper mode: discover identities, decrypt GPG keys, set env var,
/// and exec chezmoi with the original arguments.
///
/// # Errors
///
/// Returns an error if no identity files are found, no valid keys are
/// extracted, or chezmoi cannot be executed.
pub fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    // Already decrypted -> skip GPG (recursion guard)
    if env::var(ENV_AGE_KEY)
        .ok()
        .filter(|k| !k.is_empty())
        .is_some()
    {
        exec_chezmoi(&args)?;
    }

    let identities = collect_identity_paths();

    if identities.is_empty() {
        bail!(
            "no age identity files found.\n\
             Hint: set [age] identity or identities in chezmoi.toml,\n\
             or set CHEZMOI_AGE_GPG_KEY_FILE environment variable."
        );
    }

    let mut parts: Vec<SecureString> = Vec::new();
    let mut total_secret_keys: usize = 0;

    for path in &identities {
        let content = load_identity(path)?;
        let key = SecureString::new(content);
        total_secret_keys = total_secret_keys.saturating_add(key.count_secret_keys());
        parts.push(key);
    }

    if total_secret_keys == 0 {
        bail!("no valid AGE-SECRET-KEY found in any identity file");
    }

    let combined = parts
        .iter()
        .map(SecureString::as_str)
        .collect::<Vec<_>>()
        .join("\n");

    // SAFETY: This is called early in main before any threads are spawned.
    // The binary is single-threaded at this point (only main thread).
    unsafe {
        env::set_var(ENV_AGE_KEY, &combined);
    }

    tracing::info!(
        identity_count = parts.len(),
        secret_key_count = total_secret_keys,
        "loaded identity files"
    );

    drop(parts);

    exec_chezmoi(&args)?;

    Ok(())
}

/// Collect identity file paths in priority order:
///
/// 1. `chezmoi.toml` `[age]` identity/identities (excluding dummies)
/// 2. `CHEZMOI_AGE_GPG_KEY_FILE` environment variable
/// 3. Auto-scan config dirs for `*.gpg` / `*.asc` files
#[must_use]
pub fn collect_identity_paths() -> Vec<PathBuf> {
    // 1. From chezmoi.toml
    if let Some(config_path) = find() {
        tracing::debug!(config = %config_path.display(), "found chezmoi config");
        if let Ok(from_config) = read_identities(&config_path) {
            let filtered = filter_dummy_identities(from_config);
            if !filtered.is_empty() {
                return filtered;
            }
        }
    }

    // 2. From environment variable
    if let Ok(val) = env::var(ENV_GPG_KEY_FILE) {
        let paths: Vec<PathBuf> = val
            .split([',', ';'])
            .map(|s| expand_tilde(s.trim()))
            .filter(|p| p.is_file())
            .collect();
        if !paths.is_empty() {
            return paths;
        }
    }

    // 3. Auto-scan config directories
    let mut scanned = Vec::new();
    for dir in dirs() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && is_encrypted(&path) {
                    scanned.push(path);
                }
            }
        }
    }
    scanned.sort();
    scanned
}

/// Load a single identity file: GPG-decrypt if encrypted, read plaintext otherwise.
///
/// # Errors
///
/// Returns an error if the file cannot be read or GPG decryption fails.
pub fn load_identity(path: &Path) -> Result<String> {
    if is_encrypted(path) {
        tracing::info!(path = %path.display(), "gpg --decrypt");
        decrypt(path)
    } else if path.is_file() {
        let content =
            fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
        Ok(content.trim_end().to_owned())
    } else {
        bail!("identity file not found: {}", path.display())
    }
}

/// Find and exec chezmoi, passing through all arguments.
fn exec_chezmoi(args: &[String]) -> Result<()> {
    let chezmoi = find_in_path("chezmoi").context("chezmoi not found in PATH")?;
    let err = replace_process(&chezmoi, args);
    bail!("failed to exec chezmoi: {err}");
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::indexing_slicing)]

    use std::io::Write;

    use tempfile::TempDir;

    use super::*;
    use crate::test_utils::{ENV_LOCK, EnvGuard};

    #[test]
    #[cfg_attr(miri, ignore)] // uses tempfile I/O unsupported under Miri isolation
    fn test_load_identity_plaintext() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("key.txt");
        let mut f = fs::File::create(&key_path).unwrap();
        f.write_all(b"AGE-SECRET-KEY-1TESTKEY\n").unwrap();

        // Act
        let content = load_identity(&key_path).unwrap();

        // Assert
        assert_eq!(content, "AGE-SECRET-KEY-1TESTKEY");
    }

    #[test]
    #[cfg_attr(miri, ignore)] // uses filesystem access unsupported under Miri isolation
    fn test_load_identity_missing_file() {
        // Arrange
        let path = PathBuf::from("/nonexistent/path/key.txt");

        // Act
        let result = load_identity(&path);

        // Assert
        assert!(result.is_err());
    }

    /// Guard all env vars that `collect_identity_paths()` reads.
    /// Returns guards that restore on drop; caller must hold them alive.
    fn guard_collect_env() -> [EnvGuard; 5] {
        [
            EnvGuard::remove(crate::config::ENV_CHEZMOI_CONFIG),
            EnvGuard::remove(ENV_GPG_KEY_FILE),
            EnvGuard::remove("XDG_CONFIG_HOME"),
            EnvGuard::set("HOME", "/tmp/chezmage-test-nonexistent"),
            EnvGuard::remove("APPDATA"),
        ]
    }

    #[test]
    #[cfg_attr(miri, ignore)] // uses tempfile I/O unsupported under Miri isolation
    fn test_collect_identity_paths_from_env() {
        // Arrange
        let _lock = ENV_LOCK.lock().unwrap();
        let _guards = guard_collect_env();
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("key.txt");
        fs::write(&key_path, "test").unwrap();

        let _env = EnvGuard::set(ENV_GPG_KEY_FILE, key_path.to_str().unwrap());

        // Act
        let paths = collect_identity_paths();

        // Assert
        assert!(!paths.is_empty());
        assert_eq!(paths.first().unwrap(), &key_path);
    }

    #[test]
    #[cfg_attr(miri, ignore)] // uses tempfile I/O unsupported under Miri isolation
    fn test_collect_identity_paths_from_config_file() {
        // Arrange
        let _lock = ENV_LOCK.lock().unwrap();
        let _guards = guard_collect_env();
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("key.txt");
        fs::write(&key_path, "AGE-SECRET-KEY-1TESTKEY").unwrap();

        let config_path = dir.path().join("chezmoi.toml");
        fs::write(
            &config_path,
            format!("[age]\nidentity = \"{}\"\n", key_path.to_str().unwrap()),
        )
        .unwrap();

        let _config = EnvGuard::set(
            crate::config::ENV_CHEZMOI_CONFIG,
            config_path.to_str().unwrap(),
        );

        // Act
        let paths = collect_identity_paths();

        // Assert
        assert_eq!(paths, vec![key_path]);
    }

    #[test]
    #[cfg_attr(miri, ignore)] // uses tempfile I/O unsupported under Miri isolation
    fn test_collect_identity_paths_comma_separated_env() {
        // Arrange
        let _lock = ENV_LOCK.lock().unwrap();
        let _guards = guard_collect_env();
        let dir = TempDir::new().unwrap();
        let key_a = dir.path().join("a.txt");
        let key_b = dir.path().join("b.txt");
        fs::write(&key_a, "key-a").unwrap();
        fs::write(&key_b, "key-b").unwrap();

        let val = format!("{},{}", key_a.to_str().unwrap(), key_b.to_str().unwrap());
        let _env = EnvGuard::set(ENV_GPG_KEY_FILE, &val);

        // Act
        let paths = collect_identity_paths();

        // Assert
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], key_a);
        assert_eq!(paths[1], key_b);
    }

    #[test]
    #[cfg_attr(miri, ignore)] // uses tempfile I/O unsupported under Miri isolation
    fn test_collect_identity_paths_semicolon_separated_env() {
        // Arrange
        let _lock = ENV_LOCK.lock().unwrap();
        let _guards = guard_collect_env();
        let dir = TempDir::new().unwrap();
        let key_a = dir.path().join("a.txt");
        let key_b = dir.path().join("b.txt");
        fs::write(&key_a, "key-a").unwrap();
        fs::write(&key_b, "key-b").unwrap();

        let val = format!("{};{}", key_a.to_str().unwrap(), key_b.to_str().unwrap());
        let _env = EnvGuard::set(ENV_GPG_KEY_FILE, &val);

        // Act
        let paths = collect_identity_paths();

        // Assert
        assert_eq!(paths.len(), 2);
    }

    #[test]
    #[cfg_attr(miri, ignore)] // uses tempfile I/O unsupported under Miri isolation
    fn test_collect_identity_paths_auto_scan_gpg_files() {
        // Arrange
        let _lock = ENV_LOCK.lock().unwrap();
        let _guards = guard_collect_env();
        let dir = TempDir::new().unwrap();
        let chezmoi_dir = dir.path().join("chezmoi");
        fs::create_dir_all(&chezmoi_dir).unwrap();
        fs::write(chezmoi_dir.join("key.gpg"), "encrypted-key").unwrap();
        fs::write(chezmoi_dir.join("key.asc"), "encrypted-key2").unwrap();

        let _xdg = EnvGuard::set("XDG_CONFIG_HOME", dir.path().to_str().unwrap());

        // Act
        let paths = collect_identity_paths();

        // Assert
        assert_eq!(paths.len(), 2);
        assert!(
            paths
                .iter()
                .any(|p| p.to_string_lossy().contains("key.gpg"))
        );
        assert!(
            paths
                .iter()
                .any(|p| p.to_string_lossy().contains("key.asc"))
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)] // uses filesystem access unsupported under Miri isolation
    fn test_load_identity_missing_file_error_message() {
        // Arrange
        let path = PathBuf::from("/nonexistent/path/key.txt");

        // Act
        let result = load_identity(&path);

        // Assert
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("identity file not found"),
            "error message should contain 'identity file not found': {err}"
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)] // uses tempfile I/O unsupported under Miri isolation
    fn test_load_identity_plaintext_trims_trailing_whitespace() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("key.txt");
        let mut f = fs::File::create(&key_path).unwrap();
        f.write_all(b"AGE-SECRET-KEY-1TESTKEY\n  \n").unwrap();

        // Act
        let content = load_identity(&key_path).unwrap();

        // Assert
        assert_eq!(content, "AGE-SECRET-KEY-1TESTKEY");
    }
}
