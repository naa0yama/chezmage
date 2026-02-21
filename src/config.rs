//! Chezmoi configuration parsing for age identity discovery.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::exec::expand_tilde;

/// Environment variable override for chezmoi config file path.
pub const ENV_CHEZMOI_CONFIG: &str = "CHEZMOI_CONFIG";

/// Supported chezmoi config filenames in priority order.
const CONFIG_FILENAMES: &[&str] = &["chezmoi.toml"];

/// Paths treated as dummy identities by chezmoi (skipped).
const DUMMY_IDENTITIES: &[&str] = &["/dev/null", "NUL", "nul"];

/// Top-level chezmoi configuration (only the `[age]` section).
#[derive(Deserialize, Default, Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct ChezmoiConfig {
    /// Age encryption section.
    #[serde(default)]
    pub age: AgeSection,
}

/// The `[age]` section of chezmoi.toml.
#[derive(Deserialize, Default, Debug)]
pub struct AgeSection {
    /// Single identity file path.
    pub identity: Option<String>,
    /// Multiple identity file paths.
    pub identities: Option<Vec<String>>,
}

/// Locate the chezmoi config file.
///
/// Search order:
/// 1. `CHEZMOI_CONFIG` environment variable
/// 2. Standard config directories (`XDG_CONFIG_HOME/chezmoi`, `~/.config/chezmoi`)
#[must_use]
pub fn find() -> Option<PathBuf> {
    if let Ok(p) = env::var(ENV_CHEZMOI_CONFIG) {
        let path = PathBuf::from(&p);
        if path.is_file() {
            return Some(path);
        }
    }

    for dir in dirs() {
        for filename in CONFIG_FILENAMES {
            let path = dir.join(filename);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    None
}

/// Build candidate config directories from environment values.
#[must_use]
pub(crate) fn dirs_from_values(
    xdg_config_home: Option<&str>,
    home_dir: Option<PathBuf>,
    appdata: Option<&str>,
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(xdg) = xdg_config_home {
        dirs.push(PathBuf::from(xdg).join("chezmoi"));
    }
    if let Some(home) = home_dir {
        dirs.push(home.join(".config").join("chezmoi"));
    }
    if let Some(appdata) = appdata {
        dirs.push(PathBuf::from(appdata).join("chezmoi"));
    }

    dirs.dedup();
    dirs
}

/// Return candidate chezmoi config directories.
#[must_use]
pub fn dirs() -> Vec<PathBuf> {
    dirs_from_values(
        env::var("XDG_CONFIG_HOME").ok().as_deref(),
        crate::exec::home_dir().ok(),
        env::var("APPDATA").ok().as_deref(),
    )
}

/// Parse a chezmoi TOML config file and extract identity paths.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn read_identities(config_path: &Path) -> Result<Vec<PathBuf>> {
    let ext = config_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    if ext != "toml" {
        tracing::warn!(
            path = %config_path.display(),
            ext = ext,
            "only TOML config is supported"
        );
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(config_path)
        .with_context(|| format!("cannot read {}", config_path.display()))?;

    let config: ChezmoiConfig = toml::from_str(&content)
        .with_context(|| format!("parse error in {}", config_path.display()))?;

    let mut paths = Vec::new();

    if let Some(ref id) = config.age.identity {
        let id = id.trim();
        if !id.is_empty() {
            paths.push(expand_tilde(id));
        }
    }

    if let Some(ref ids) = config.age.identities {
        for id in ids {
            let id = id.trim();
            if !id.is_empty() {
                paths.push(expand_tilde(id));
            }
        }
    }

    Ok(paths)
}

/// Filter out dummy identity paths (`/dev/null`, `NUL`).
#[must_use]
pub fn filter_dummy_identities(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths
        .into_iter()
        .filter(|p| {
            let s = p.to_string_lossy();
            !DUMMY_IDENTITIES.iter().any(|d| s.as_ref() == *d)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    #![allow(clippy::indexing_slicing)]

    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;
    use crate::test_utils::{ENV_LOCK, EnvGuard};

    fn write_toml(content: &str) -> NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn test_read_config_single_identity() {
        // Arrange
        let f = write_toml(
            r#"
[age]
identity = "/home/user/.config/chezmoi/key.gpg"
"#,
        );

        // Act
        let paths = read_identities(f.path()).unwrap();

        // Assert
        assert_eq!(paths.len(), 1);
        assert_eq!(
            paths.first().unwrap().to_str().unwrap(),
            "/home/user/.config/chezmoi/key.gpg"
        );
    }

    #[test]
    fn test_read_config_multiple_identities() {
        // Arrange
        let f = write_toml(
            r#"
[age]
identities = ["/path/a.gpg", "/path/b.asc"]
"#,
        );

        // Act
        let paths = read_identities(f.path()).unwrap();

        // Assert
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_read_config_both_identity_and_identities() {
        // Arrange
        let f = write_toml(
            r#"
[age]
identity = "/path/main.gpg"
identities = ["/path/sub.gpg"]
"#,
        );

        // Act
        let paths = read_identities(f.path()).unwrap();

        // Assert
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn test_read_config_empty_age_section() {
        // Arrange
        let f = write_toml(
            r"
[age]
",
        );

        // Act
        let paths = read_identities(f.path()).unwrap();

        // Assert
        assert!(paths.is_empty());
    }

    #[test]
    fn test_read_config_no_age_section() {
        // Arrange
        let f = write_toml(
            r#"
[data]
name = "test"
"#,
        );

        // Act
        let paths = read_identities(f.path()).unwrap();

        // Assert
        assert!(paths.is_empty());
    }

    #[test]
    fn test_read_config_invalid_toml() {
        // Arrange
        let f = write_toml("this is not valid toml {{{}}}");

        // Act
        let result = read_identities(f.path());

        // Assert
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("parse error"),
            "error should contain 'parse error': {err}"
        );
    }

    #[test]
    fn test_read_config_non_toml_extension() {
        // Arrange
        let mut f = tempfile::Builder::new().suffix(".yaml").tempfile().unwrap();
        f.write_all(b"age:\n  identity: key.gpg\n").unwrap();
        f.flush().unwrap();

        // Act
        let paths = read_identities(f.path()).unwrap();

        // Assert
        assert!(paths.is_empty());
    }

    #[test]
    fn test_read_config_tilde_expansion() {
        // Arrange
        let f = write_toml(
            r#"
[age]
identity = "~/key.gpg"
"#,
        );

        // Act
        let paths = read_identities(f.path()).unwrap();

        // Assert
        assert_eq!(paths.len(), 1);
        let path_str = paths.first().unwrap().to_str().unwrap();
        assert!(!path_str.starts_with('~'));
    }

    #[test]
    fn test_filter_dummy_identities_removes_dev_null() {
        // Arrange
        let paths = vec![PathBuf::from("/dev/null"), PathBuf::from("/path/key.gpg")];

        // Act
        let filtered = filter_dummy_identities(paths);

        // Assert
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.first().unwrap().to_str().unwrap(), "/path/key.gpg");
    }

    #[test]
    fn test_filter_dummy_identities_removes_nul() {
        // Arrange
        let paths = vec![
            PathBuf::from("NUL"),
            PathBuf::from("nul"),
            PathBuf::from("/path/key.gpg"),
        ];

        // Act
        let filtered = filter_dummy_identities(paths);

        // Assert
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_filter_dummy_identities_keeps_all() {
        // Arrange
        let paths = vec![PathBuf::from("/path/a.gpg"), PathBuf::from("/path/b.asc")];

        // Act
        let filtered = filter_dummy_identities(paths);

        // Assert
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_find_from_env_var() {
        // Arrange
        let _lock = ENV_LOCK.lock().unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        let config_path = dir.path().join("chezmoi.toml");
        std::fs::write(&config_path, "[age]\n").unwrap();
        let _guard = EnvGuard::set(ENV_CHEZMOI_CONFIG, config_path.to_str().unwrap());

        // Act
        let result = find();

        // Assert
        assert_eq!(result, Some(config_path));
    }

    #[test]
    fn test_find_from_env_var_nonexistent() {
        // Arrange
        let _lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::set(ENV_CHEZMOI_CONFIG, "/nonexistent/chezmoi.toml");
        let _home = EnvGuard::set("HOME", "/tmp/chezmage-test-nonexistent");
        let _xdg = EnvGuard::remove("XDG_CONFIG_HOME");
        let _appdata = EnvGuard::remove("APPDATA");

        // Act
        let result = find();

        // Assert: invalid path falls through, no config found
        assert!(result.is_none());
    }

    #[test]
    fn test_find_from_xdg_dir() {
        // Arrange
        let _lock = ENV_LOCK.lock().unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        let chezmoi_dir = dir.path().join("chezmoi");
        std::fs::create_dir_all(&chezmoi_dir).unwrap();
        let config_path = chezmoi_dir.join("chezmoi.toml");
        std::fs::write(&config_path, "[age]\n").unwrap();

        let _env_guard = EnvGuard::remove(ENV_CHEZMOI_CONFIG);
        let _xdg = EnvGuard::set("XDG_CONFIG_HOME", dir.path().to_str().unwrap());

        // Act
        let result = find();

        // Assert
        assert_eq!(result, Some(config_path));
    }

    #[test]
    fn test_find_returns_none_when_nothing_exists() {
        // Arrange
        let _lock = ENV_LOCK.lock().unwrap();
        let _env_guard = EnvGuard::remove(ENV_CHEZMOI_CONFIG);
        let _home = EnvGuard::set("HOME", "/tmp/chezmage-test-nonexistent");
        let _xdg = EnvGuard::remove("XDG_CONFIG_HOME");
        let _appdata = EnvGuard::remove("APPDATA");

        // Act
        let result = find();

        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_read_identities_empty_string_identity() {
        // Arrange
        let f = write_toml(
            r#"
[age]
identity = ""
"#,
        );

        // Act
        let paths = read_identities(f.path()).unwrap();

        // Assert
        assert!(paths.is_empty());
    }

    #[test]
    fn test_read_identities_whitespace_only_identity() {
        // Arrange
        let f = write_toml(
            r#"
[age]
identity = "  "
"#,
        );

        // Act
        let paths = read_identities(f.path()).unwrap();

        // Assert
        assert!(paths.is_empty());
    }

    #[test]
    fn test_dirs_from_values_xdg_only() {
        // Arrange & Act
        let result = dirs_from_values(Some("/xdg/config"), None, None);

        // Assert
        assert_eq!(result, vec![PathBuf::from("/xdg/config/chezmoi")]);
    }

    #[test]
    fn test_dirs_from_values_home_only() {
        // Arrange & Act
        let result = dirs_from_values(None, Some(PathBuf::from("/home/user")), None);

        // Assert
        assert_eq!(result, vec![PathBuf::from("/home/user/.config/chezmoi")]);
    }

    #[test]
    fn test_dirs_from_values_all_set() {
        // Arrange & Act
        let result = dirs_from_values(
            Some("/xdg/config"),
            Some(PathBuf::from("/home/user")),
            Some("/appdata"),
        );

        // Assert
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], PathBuf::from("/xdg/config/chezmoi"));
        assert_eq!(result[1], PathBuf::from("/home/user/.config/chezmoi"));
        assert_eq!(result[2], PathBuf::from("/appdata/chezmoi"));
    }

    #[test]
    fn test_dirs_from_values_dedup() {
        // Arrange: XDG and HOME resolve to the same path
        let result = dirs_from_values(
            Some("/home/user/.config"),
            Some(PathBuf::from("/home/user")),
            None,
        );

        // Assert: consecutive duplicates are removed by dedup
        assert_eq!(result, vec![PathBuf::from("/home/user/.config/chezmoi")]);
    }
}
