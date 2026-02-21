//! Process execution and path utilities.

use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

/// Environment variable holding pre-decrypted age key material.
pub const ENV_AGE_KEY: &str = "CHEZMOI_AGE_KEY";

/// Environment variable for explicit GPG key file path(s).
pub const ENV_GPG_KEY_FILE: &str = "CHEZMOI_AGE_GPG_KEY_FILE";

/// Expand leading `~/` or `~\` to the user's home directory.
#[must_use]
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\"))
        && let Ok(home) = home_dir()
    {
        return home.join(rest);
    }
    if path == "~"
        && let Ok(home) = home_dir()
    {
        return home;
    }
    PathBuf::from(path)
}

/// Get the user's home directory from environment variables.
///
/// # Errors
///
/// Returns an error if neither `HOME` nor `USERPROFILE` is set.
pub fn home_dir() -> Result<PathBuf> {
    env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map(PathBuf::from)
        .context("neither HOME nor USERPROFILE environment variable is set")
}

/// Search for a binary by name on PATH.
#[must_use]
pub fn find_in_path(name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    let exe_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        String::from(name)
    };

    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(&exe_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Search directories for the age binary, skipping `self_path` if present.
#[must_use]
pub(crate) fn find_age_in_dirs(
    dirs: &[PathBuf],
    exe_name: &str,
    self_path: Option<&Path>,
) -> Option<PathBuf> {
    for dir in dirs {
        let candidate = dir.join(exe_name);
        if candidate.is_file() {
            if let Some(self_p) = self_path
                && let Ok(resolved) = candidate.canonicalize()
                && resolved == self_p
            {
                continue;
            }
            return Some(candidate);
        }
    }
    None
}

/// Find the real `age` binary on PATH, excluding our own executable.
///
/// # Errors
///
/// Returns an error if no `age` binary is found.
pub fn find_real_age() -> Result<PathBuf> {
    let self_path = env::current_exe().ok().and_then(|p| p.canonicalize().ok());

    let exe_name = if cfg!(windows) { "age.exe" } else { "age" };

    let dirs: Vec<PathBuf> = env::var_os("PATH")
        .map(|p| env::split_paths(&p).collect())
        .unwrap_or_default();

    find_age_in_dirs(&dirs, exe_name, self_path.as_deref())
        .ok_or_else(|| anyhow::anyhow!("age binary not found in PATH"))
}

/// Replace the current process with the given program (Unix: execvp).
///
/// On Unix this never returns on success. On Windows it spawns and exits.
#[must_use]
pub fn replace_process(program: &Path, args: &[String]) -> io::Error {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: execvp replaces the process image. On success this never
        // returns. The CommandExt::exec() method is safe Rust wrapping execvp.
        Command::new(program).args(args).exec()
    }

    #[cfg(not(unix))]
    {
        match Command::new(program).args(args).status() {
            Ok(status) => {
                #[allow(clippy::exit)]
                std::process::exit(status.code().unwrap_or(1));
            }
            Err(e) => e,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::similar_names)]

    use super::*;
    use crate::test_utils::{ENV_LOCK, EnvGuard};

    #[test]
    fn test_expand_tilde_with_path() {
        // Arrange
        let home = env::var("HOME").unwrap();

        // Act
        let result = expand_tilde("~/Documents/file.txt");

        // Assert
        let expected = PathBuf::from(home).join("Documents/file.txt");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_expand_tilde_standalone() {
        // Arrange
        let home = env::var("HOME").unwrap();

        // Act
        let result = expand_tilde("~");

        // Assert
        assert_eq!(result, PathBuf::from(home));
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        // Arrange & Act
        let result = expand_tilde("/absolute/path");

        // Assert
        assert_eq!(result, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_expand_tilde_relative() {
        // Arrange & Act
        let result = expand_tilde("relative/path");

        // Assert
        assert_eq!(result, PathBuf::from("relative/path"));
    }

    #[test]
    fn test_home_dir_returns_path() {
        // Arrange & Act
        let result = home_dir();

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_in_path_existing_binary() {
        // Arrange & Act
        let result = find_in_path("sh");

        // Assert (sh should exist on any Unix-like system)
        if cfg!(unix) {
            assert!(result.is_some());
        }
    }

    #[test]
    fn test_find_in_path_nonexistent() {
        // Arrange & Act
        let result = find_in_path("nonexistent_binary_xyz_123");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_find_age_in_dirs_finds_binary() {
        // Arrange
        let dir = tempfile::TempDir::new().unwrap();
        let bin_path = dir.path().join("age");
        std::fs::write(&bin_path, "fake-age").unwrap();

        let dirs = vec![dir.path().to_path_buf()];

        // Act
        let result = find_age_in_dirs(&dirs, "age", None);

        // Assert
        assert_eq!(result, Some(bin_path));
    }

    #[test]
    fn test_find_age_in_dirs_skips_self() {
        // Arrange
        let dir = tempfile::TempDir::new().unwrap();
        let bin_path = dir.path().join("age");
        std::fs::write(&bin_path, "fake-age").unwrap();

        let self_path = bin_path.canonicalize().unwrap();
        let dirs = vec![dir.path().to_path_buf()];

        // Act
        let result = find_age_in_dirs(&dirs, "age", Some(&self_path));

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_find_age_in_dirs_empty_dirs() {
        // Arrange
        let dirs: Vec<PathBuf> = Vec::new();

        // Act
        let result = find_age_in_dirs(&dirs, "age", None);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_find_age_in_dirs_no_match() {
        // Arrange
        let dir = tempfile::TempDir::new().unwrap();
        let dirs = vec![dir.path().to_path_buf()];

        // Act
        let result = find_age_in_dirs(&dirs, "age", None);

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_find_age_in_dirs_picks_first() {
        // Arrange
        let dir1 = tempfile::TempDir::new().unwrap();
        let dir2 = tempfile::TempDir::new().unwrap();
        let bin1 = dir1.path().join("age");
        let bin2 = dir2.path().join("age");
        std::fs::write(&bin1, "age-first").unwrap();
        std::fs::write(&bin2, "age-second").unwrap();

        let dirs = vec![dir1.path().to_path_buf(), dir2.path().to_path_buf()];

        // Act
        let result = find_age_in_dirs(&dirs, "age", None);

        // Assert
        assert_eq!(result, Some(bin1));
    }

    #[test]
    fn test_expand_tilde_with_backslash() {
        // Arrange
        let home = env::var("HOME").unwrap();

        // Act
        let result = expand_tilde("~\\Documents\\file.txt");

        // Assert
        let expected = PathBuf::from(home).join("Documents\\file.txt");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_find_in_path_returns_none_when_path_unset() {
        // Arrange
        let _lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::remove("PATH");

        // Act
        let result = find_in_path("sh");

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_home_dir_falls_back_to_userprofile() {
        // Arrange
        let _lock = ENV_LOCK.lock().unwrap();
        let _home = EnvGuard::remove("HOME");
        let _userprofile = EnvGuard::set("USERPROFILE", "/fake/userprofile");

        // Act
        let result = home_dir();

        // Assert
        assert_eq!(result.unwrap(), PathBuf::from("/fake/userprofile"));
    }
}
