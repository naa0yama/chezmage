//! Wrapper mode: read chezmoi config, decrypt GPG identities, exec chezmoi.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::config::{dirs, filter_dummy_identities, find, read_identities};
use crate::exec::{ENV_AGE_KEY, ENV_GPG_KEY_FILE, expand_tilde, find_in_path, replace_process};
use crate::gpg::{decrypt, is_encrypted};
use crate::secure::SecureString;

/// Chezmoi subcommands that never need age decryption.
///
/// These commands deal only with metadata, config, shell operations,
/// or write-only encryption (using the age public key / recipient),
/// and never read encrypted file content.  Unknown subcommands always
/// trigger decryption as a safe default.
///
/// Derived from chezmoi source (`internal/cmd/`):
/// - Commands that never call `getSourceState()` / `newSourceState()`
/// - Commands that use `makeRunEWithSourceState()` but only inspect
///   metadata (paths / attributes), never calling `Contents()` which
///   would trigger lazy `AgeEncryption.Decrypt()`.
const PASSTHROUGH_SUBCOMMANDS: &[&str] = &[
    "add",
    "age-keygen",
    "cat-config",
    "cd",
    "chattr",
    "completion",
    "data",
    "doctor",
    "dump-config",
    "edit-config",
    "edit-config-template",
    "encrypt",
    "execute-template",
    "forget",
    "generate",
    "git",
    "help",
    "ignored",
    "license",
    "managed",
    "purge",
    "re-add",
    "secret",
    "source-path",
    "state",
    "target-path",
    "unmanaged",
    "upgrade",
];

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

    // Skip decryption for subcommands that never need age private keys
    if !needs_decryption(&args) {
        tracing::debug!(
            subcommand = ?extract_subcommand(&args),
            "skipping GPG decryption",
        );
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

/// Extract the chezmoi subcommand from the argument list.
///
/// Returns the first argument that does not start with `-`.
fn extract_subcommand(args: &[String]) -> Option<&str> {
    args.iter()
        .find(|a| !a.starts_with('-'))
        .map(String::as_str)
}

/// Check whether `--exclude`/`-x` flags exclude `encrypted` entries.
///
/// Scans arguments left-to-right, tracking whether `encrypted` is currently
/// excluded.  Later flags override earlier ones (matching chezmoi semantics).
/// Returns `false` (safe default: do decrypt) for ambiguous or empty input.
fn excludes_encrypted(args: &[String]) -> bool {
    let mut excluded = false;
    let mut skip_next = false;

    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        // Determine flag kind and its value(s)
        let (is_exclude, values) = if let Some(val) = arg.strip_prefix("--exclude=") {
            (true, val)
        } else if let Some(val) = arg.strip_prefix("--include=") {
            (false, val)
        } else if let Some(val) = arg.strip_prefix("-x=") {
            (true, val)
        } else if let Some(val) = arg.strip_prefix("-i=") {
            (false, val)
        } else if arg == "--exclude" || arg == "-x" {
            match i.checked_add(1).and_then(|j| args.get(j)) {
                Some(next) if !next.starts_with('-') => {
                    skip_next = true;
                    (true, next.as_str())
                }
                _ => continue, // missing value — skip
            }
        } else if arg == "--include" || arg == "-i" {
            match i.checked_add(1).and_then(|j| args.get(j)) {
                Some(next) if !next.starts_with('-') => {
                    skip_next = true;
                    (false, next.as_str())
                }
                _ => continue,
            }
        } else {
            continue;
        };

        // Parse comma-separated entry types
        for entry in values.split(',') {
            let entry = entry.trim();
            match (is_exclude, entry) {
                (true, "encrypted" | "all") | (false, "none" | "noencrypted") => excluded = true,
                (true, "none" | "noencrypted") | (false, "encrypted" | "all") => excluded = false,
                _ => {}
            }
        }
    }

    excluded
}

/// Check whether the given arguments require age key decryption.
///
/// Returns `false` for known passthrough subcommands that never read
/// encrypted content, or when `--exclude encrypted` is specified.
/// Returns `true` for everything else (safe default).
fn needs_decryption(args: &[String]) -> bool {
    if extract_subcommand(args).is_some_and(|cmd| PASSTHROUGH_SUBCOMMANDS.contains(&cmd)) {
        return false;
    }
    if excludes_encrypted(args) {
        return false;
    }
    true
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

    // -----------------------------------------------------------------
    // extract_subcommand / needs_decryption
    // -----------------------------------------------------------------

    fn args(vals: &[&str]) -> Vec<String> {
        vals.iter().map(|s| String::from(*s)).collect()
    }

    #[test]
    fn test_extract_subcommand_simple() {
        // Arrange
        let a = args(&["doctor"]);

        // Act & Assert
        assert_eq!(extract_subcommand(&a), Some("doctor"));
    }

    #[test]
    fn test_extract_subcommand_with_flags_before() {
        // Arrange
        let a = args(&["--color", "false", "managed"]);

        // Act & Assert
        assert_eq!(extract_subcommand(&a), Some("false"));
    }

    #[test]
    fn test_extract_subcommand_empty_args() {
        // Arrange
        let a: Vec<String> = vec![];

        // Act & Assert
        assert_eq!(extract_subcommand(&a), None);
    }

    #[test]
    fn test_extract_subcommand_only_flags() {
        // Arrange
        let a = args(&["--verbose", "--debug"]);

        // Act & Assert
        assert_eq!(extract_subcommand(&a), None);
    }

    #[test]
    fn test_extract_subcommand_with_flags_after() {
        // Arrange
        let a = args(&["apply", "--verbose"]);

        // Act & Assert
        assert_eq!(extract_subcommand(&a), Some("apply"));
    }

    #[test]
    fn test_needs_decryption_apply() {
        // Arrange & Act & Assert
        assert!(needs_decryption(&args(&["apply"])));
    }

    #[test]
    fn test_needs_decryption_diff() {
        // Arrange & Act & Assert
        assert!(needs_decryption(&args(&["diff"])));
    }

    #[test]
    fn test_needs_decryption_cd() {
        // Arrange & Act & Assert
        assert!(!needs_decryption(&args(&["cd"])));
    }

    #[test]
    fn test_needs_decryption_doctor() {
        // Arrange & Act & Assert
        assert!(!needs_decryption(&args(&["doctor"])));
    }

    #[test]
    fn test_needs_decryption_managed() {
        // Arrange & Act & Assert
        assert!(!needs_decryption(&args(&["managed"])));
    }

    #[test]
    fn test_needs_decryption_empty_args() {
        // Arrange & Act & Assert
        assert!(needs_decryption(&args(&[])));
    }

    #[test]
    fn test_needs_decryption_unknown_command() {
        // Arrange & Act & Assert
        assert!(needs_decryption(&args(&["unknown-cmd"])));
    }

    #[test]
    fn test_needs_decryption_with_flags_before_passthrough() {
        // Arrange — first non-flag arg is "false", which is unknown
        let a = args(&["--color", "false", "doctor"]);

        // Act & Assert
        assert!(needs_decryption(&a));
    }

    #[test]
    fn test_needs_decryption_all_passthrough_subcommands() {
        // Arrange
        let all = [
            "add",
            "age-keygen",
            "cat-config",
            "cd",
            "chattr",
            "completion",
            "data",
            "doctor",
            "dump-config",
            "edit-config",
            "edit-config-template",
            "encrypt",
            "execute-template",
            "forget",
            "generate",
            "git",
            "help",
            "ignored",
            "license",
            "managed",
            "purge",
            "re-add",
            "secret",
            "source-path",
            "state",
            "target-path",
            "unmanaged",
            "upgrade",
        ];

        // Act & Assert
        for cmd in all {
            assert!(
                !needs_decryption(&args(&[cmd])),
                "{cmd} should be passthrough"
            );
        }
    }

    #[test]
    fn test_needs_decryption_add() {
        // Arrange & Act & Assert
        assert!(!needs_decryption(&args(&["add"])));
    }

    #[test]
    fn test_needs_decryption_add_encrypt() {
        // Arrange & Act & Assert
        assert!(!needs_decryption(&args(&["add", "--encrypt"])));
    }

    #[test]
    fn test_needs_decryption_re_add() {
        // Arrange & Act & Assert
        assert!(!needs_decryption(&args(&["re-add"])));
    }

    #[test]
    fn test_needs_decryption_encrypt() {
        // Arrange & Act & Assert
        assert!(!needs_decryption(&args(&["encrypt"])));
    }

    // -----------------------------------------------------------------
    // excludes_encrypted
    // -----------------------------------------------------------------

    #[test]
    fn test_excludes_encrypted_long_space() {
        // Arrange & Act & Assert
        assert!(excludes_encrypted(&args(&["--exclude", "encrypted"])));
    }

    #[test]
    fn test_excludes_encrypted_long_equals() {
        // Arrange & Act & Assert
        assert!(excludes_encrypted(&args(&["--exclude=encrypted"])));
    }

    #[test]
    fn test_excludes_encrypted_short_space() {
        // Arrange & Act & Assert
        assert!(excludes_encrypted(&args(&["-x", "encrypted"])));
    }

    #[test]
    fn test_excludes_encrypted_short_equals() {
        // Arrange & Act & Assert
        assert!(excludes_encrypted(&args(&["-x=encrypted"])));
    }

    #[test]
    fn test_excludes_encrypted_comma_separated() {
        // Arrange & Act & Assert
        assert!(excludes_encrypted(&args(&["--exclude", "dirs,encrypted"])));
    }

    #[test]
    fn test_excludes_encrypted_comma_without_encrypted() {
        // Arrange & Act & Assert
        assert!(!excludes_encrypted(&args(&["--exclude", "dirs,files"])));
    }

    #[test]
    fn test_excludes_encrypted_all_keyword() {
        // Arrange & Act & Assert
        assert!(excludes_encrypted(&args(&["--exclude", "all"])));
    }

    #[test]
    fn test_excludes_encrypted_none_keyword() {
        // Arrange & Act & Assert
        assert!(!excludes_encrypted(&args(&["--exclude", "none"])));
    }

    #[test]
    fn test_excludes_encrypted_noencrypted_in_exclude() {
        // --exclude noencrypted means "don't exclude encrypted" -> false
        assert!(!excludes_encrypted(&args(&["--exclude", "noencrypted"])));
    }

    #[test]
    fn test_excludes_encrypted_include_re_enables() {
        // --exclude encrypted then --include encrypted -> not excluded
        assert!(!excludes_encrypted(&args(&[
            "--exclude",
            "encrypted",
            "--include",
            "encrypted",
        ])));
    }

    #[test]
    fn test_excludes_encrypted_include_all_re_enables() {
        // --exclude encrypted then --include all -> not excluded
        assert!(!excludes_encrypted(&args(&[
            "--exclude",
            "encrypted",
            "--include",
            "all",
        ])));
    }

    #[test]
    fn test_excludes_encrypted_include_none_excludes() {
        // --include none -> same as exclude all
        assert!(excludes_encrypted(&args(&["-i", "none"])));
    }

    #[test]
    fn test_excludes_encrypted_include_noencrypted() {
        // --include noencrypted means "include the negation" -> excluded
        assert!(excludes_encrypted(&args(&["-i", "noencrypted"])));
    }

    #[test]
    fn test_excludes_encrypted_empty_args() {
        // Arrange & Act & Assert
        assert!(!excludes_encrypted(&args(&[])));
    }

    #[test]
    fn test_excludes_encrypted_unrelated_flags() {
        // Arrange & Act & Assert
        assert!(!excludes_encrypted(&args(&[
            "--verbose",
            "--color",
            "false",
            "apply",
        ])));
    }

    #[test]
    fn test_excludes_encrypted_exclude_dirs_only() {
        // Arrange & Act & Assert
        assert!(!excludes_encrypted(&args(&["--exclude", "dirs"])));
    }

    #[test]
    fn test_excludes_encrypted_missing_value() {
        // --exclude with no value should not panic and return false
        assert!(!excludes_encrypted(&args(&["--exclude"])));
    }

    #[test]
    fn test_excludes_encrypted_missing_value_at_end_with_flag_next() {
        // --exclude followed by another flag — should skip
        assert!(!excludes_encrypted(&args(&["--exclude", "--verbose"])));
    }

    #[test]
    fn test_excludes_encrypted_last_flag_wins() {
        // --include encrypted then --exclude encrypted -> excluded
        assert!(excludes_encrypted(&args(&[
            "--include",
            "encrypted",
            "--exclude",
            "encrypted",
        ])));
    }

    // -----------------------------------------------------------------
    // needs_decryption + --exclude
    // -----------------------------------------------------------------

    #[test]
    fn test_needs_decryption_exclude_encrypted() {
        // Arrange & Act & Assert
        assert!(!needs_decryption(&args(&[
            "status",
            "--exclude",
            "encrypted",
        ])));
    }

    #[test]
    fn test_needs_decryption_exclude_dirs_still_needs() {
        // Arrange & Act & Assert
        assert!(needs_decryption(&args(&["status", "--exclude", "dirs"])));
    }

    #[test]
    fn test_needs_decryption_exclude_encrypted_short() {
        // Arrange & Act & Assert
        assert!(!needs_decryption(&args(&["apply", "-x=encrypted",])));
    }

    #[test]
    fn test_needs_decryption_passthrough_with_exclude() {
        // passthrough subcommand still skips regardless of exclude flag
        assert!(!needs_decryption(&args(&[
            "doctor",
            "--exclude",
            "encrypted",
        ])));
    }

    #[test]
    fn test_needs_decryption_exclude_then_include_needs() {
        // --exclude encrypted then --include encrypted -> needs decryption
        assert!(needs_decryption(&args(&[
            "apply",
            "--exclude",
            "encrypted",
            "--include",
            "encrypted",
        ])));
    }

    #[test]
    fn test_needs_decryption_exclude_all() {
        // --exclude all -> excludes encrypted -> no decryption needed
        assert!(!needs_decryption(&args(&["diff", "-x", "all"])));
    }

    #[test]
    fn test_needs_decryption_commands_requiring_decryption() {
        // Arrange
        let all = [
            "age",
            "apply",
            "archive",
            "cat",
            "decrypt",
            "destroy",
            "diff",
            "docker",
            "dump",
            "edit",
            "edit-encrypted",
            "import",
            "init",
            "merge",
            "merge-all",
            "ssh",
            "status",
            "update",
            "verify",
        ];

        // Act & Assert
        for cmd in all {
            assert!(
                needs_decryption(&args(&[cmd])),
                "{cmd} should require decryption"
            );
        }
    }
}
