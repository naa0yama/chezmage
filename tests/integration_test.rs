#![allow(clippy::unwrap_used)]
#![allow(missing_docs)]

use assert_cmd::cargo_bin_cmd;
use predicates::prelude::predicate;
use tempfile::TempDir;

#[test]
#[cfg_attr(miri, ignore)]
fn test_cli_version_flag() {
    // Arrange & Act & Assert
    let mut cmd = cargo_bin_cmd!("chezmage");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("chezmage"));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_cli_help_flag() {
    // Arrange & Act & Assert
    let mut cmd = cargo_bin_cmd!("chezmage");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("chezmage"));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_cli_no_config_no_env_shows_error() {
    // Arrange & Act & Assert
    let mut cmd = cargo_bin_cmd!("chezmage");
    cmd.env_remove("CHEZMOI_CONFIG")
        .env_remove("CHEZMOI_AGE_KEY")
        .env_remove("CHEZMOI_AGE_GPG_KEY_FILE")
        .env_remove("XDG_CONFIG_HOME")
        .env("HOME", "/tmp/chezmage-test-nonexistent")
        .assert()
        .failure()
        .stdout(predicate::str::contains("no age identity files found"));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_cli_invalid_config_file() {
    // Arrange
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("chezmoi.toml");
    std::fs::write(&config_path, "this is {{invalid}} toml").unwrap();

    // Act & Assert
    let mut cmd = cargo_bin_cmd!("chezmage");
    cmd.env("CHEZMOI_CONFIG", config_path.to_str().unwrap())
        .env_remove("CHEZMOI_AGE_KEY")
        .env_remove("CHEZMOI_AGE_GPG_KEY_FILE")
        .env("HOME", "/tmp/chezmage-test-nonexistent")
        .assert()
        .failure();
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_cli_config_with_dummy_identity() {
    // Arrange: config with /dev/null identity should be filtered out
    let dir = tempfile::TempDir::new().unwrap();
    let config_path = dir.path().join("chezmoi.toml");
    std::fs::write(&config_path, "[age]\nidentity = \"/dev/null\"\n").unwrap();

    // Act & Assert
    let mut cmd = cargo_bin_cmd!("chezmage");
    cmd.env("CHEZMOI_CONFIG", config_path.to_str().unwrap())
        .env_remove("CHEZMOI_AGE_KEY")
        .env_remove("CHEZMOI_AGE_GPG_KEY_FILE")
        .env("HOME", "/tmp/chezmage-test-nonexistent")
        .assert()
        .failure()
        .stdout(predicate::str::contains("no age identity files found"));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_cli_config_with_plaintext_identity_no_valid_key() {
    // Arrange: plaintext identity with no AGE-SECRET-KEY lines
    let dir = tempfile::TempDir::new().unwrap();
    let key_path = dir.path().join("key.txt");
    std::fs::write(&key_path, "not a valid key\n").unwrap();

    let config_path = dir.path().join("chezmoi.toml");
    std::fs::write(
        &config_path,
        format!("[age]\nidentity = \"{}\"\n", key_path.to_str().unwrap()),
    )
    .unwrap();

    // Act & Assert
    let mut cmd = cargo_bin_cmd!("chezmage");
    cmd.env("CHEZMOI_CONFIG", config_path.to_str().unwrap())
        .env_remove("CHEZMOI_AGE_KEY")
        .env_remove("CHEZMOI_AGE_GPG_KEY_FILE")
        .env("HOME", "/tmp/chezmage-test-nonexistent")
        .assert()
        .failure()
        .stdout(predicate::str::contains("no valid AGE-SECRET-KEY"));
}

// -------------------------------------------------------------------------
// Phase 3A: Wrapper mode integration tests
// -------------------------------------------------------------------------

#[test]
#[cfg_attr(miri, ignore)]
fn test_cli_recursion_guard_skips_gpg() {
    // Arrange: CHEZMOI_AGE_KEY already set — should skip GPG and try to exec chezmoi
    let mut cmd = cargo_bin_cmd!("chezmage");
    cmd.env("CHEZMOI_AGE_KEY", "AGE-SECRET-KEY-1FAKEKEY")
        .env_remove("CHEZMOI_CONFIG")
        .env_remove("CHEZMOI_AGE_GPG_KEY_FILE")
        .env("HOME", "/tmp/chezmage-test-nonexistent")
        .env("PATH", "/tmp/chezmage-test-nonexistent");

    // Act & Assert: should fail with "chezmoi not found" (not "no age identity")
    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("chezmoi not found"));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_cli_config_priority_over_env() {
    // Arrange: config file identity takes priority over env var
    let dir = TempDir::new().unwrap();
    let key_path = dir.path().join("key.txt");
    std::fs::write(&key_path, "AGE-SECRET-KEY-1FROMCONFIG\n").unwrap();

    let config_path = dir.path().join("chezmoi.toml");
    std::fs::write(
        &config_path,
        format!("[age]\nidentity = \"{}\"\n", key_path.to_str().unwrap()),
    )
    .unwrap();

    let env_key = dir.path().join("env-key.txt");
    std::fs::write(&env_key, "AGE-SECRET-KEY-1FROMENV\n").unwrap();

    // Act & Assert: with valid config identity + valid env, should use config
    // and try to exec chezmoi (fail because chezmoi is not in PATH)
    let mut cmd = cargo_bin_cmd!("chezmage");
    cmd.env("CHEZMOI_CONFIG", config_path.to_str().unwrap())
        .env("CHEZMOI_AGE_GPG_KEY_FILE", env_key.to_str().unwrap())
        .env_remove("CHEZMOI_AGE_KEY")
        .env("HOME", "/tmp/chezmage-test-nonexistent")
        .env("PATH", "/tmp/chezmage-test-nonexistent")
        .assert()
        .failure()
        .stdout(predicate::str::contains("chezmoi not found"));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_cli_valid_key_attempts_chezmoi_exec() {
    // Arrange: valid key file, no chezmoi in PATH
    let dir = TempDir::new().unwrap();
    let key_path = dir.path().join("key.txt");
    std::fs::write(&key_path, "AGE-SECRET-KEY-1VALIDTESTKEY\n").unwrap();

    let config_path = dir.path().join("chezmoi.toml");
    std::fs::write(
        &config_path,
        format!("[age]\nidentity = \"{}\"\n", key_path.to_str().unwrap()),
    )
    .unwrap();

    // Act & Assert: should load key and try to exec chezmoi
    let mut cmd = cargo_bin_cmd!("chezmage");
    cmd.env("CHEZMOI_CONFIG", config_path.to_str().unwrap())
        .env_remove("CHEZMOI_AGE_KEY")
        .env_remove("CHEZMOI_AGE_GPG_KEY_FILE")
        .env("HOME", "/tmp/chezmage-test-nonexistent")
        .env("PATH", "/tmp/chezmage-test-nonexistent")
        .assert()
        .failure()
        .stdout(predicate::str::contains("chezmoi not found"));
}

// -------------------------------------------------------------------------
// Phase 3B: Shim mode integration tests
// -------------------------------------------------------------------------

#[test]
#[cfg_attr(miri, ignore)]
fn test_shim_no_age_key_falls_back() {
    // Arrange & Act & Assert: no CHEZMOI_AGE_KEY, no age in PATH
    let mut cmd = cargo_bin_cmd!("chezmage");
    cmd.arg("--shim")
        .env_remove("CHEZMOI_AGE_KEY")
        .env("PATH", "/tmp/chezmage-test-nonexistent")
        .assert()
        .failure()
        .stdout(predicate::str::contains("age binary not found"));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_shim_with_key_no_identity_flag_falls_back() {
    // Arrange & Act & Assert: CHEZMOI_AGE_KEY is set but no -i flag → should fallback
    let mut cmd = cargo_bin_cmd!("chezmage");
    cmd.args(["--shim", "-e", "somefile.txt"])
        .env("CHEZMOI_AGE_KEY", "AGE-SECRET-KEY-1FAKEKEY")
        .env("PATH", "/tmp/chezmage-test-nonexistent")
        .assert()
        .failure()
        .stdout(predicate::str::contains("age binary not found"));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_shim_flag_detection() {
    // Arrange & Act & Assert: --shim triggers shim mode, --version is passed to age
    let mut cmd = cargo_bin_cmd!("chezmage");
    cmd.args(["--shim", "--version"])
        .env_remove("CHEZMOI_AGE_KEY")
        .env("PATH", "/tmp/chezmage-test-nonexistent")
        .assert()
        .failure()
        .stdout(predicate::str::contains("age binary not found"));
}
