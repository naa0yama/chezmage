#![allow(clippy::unwrap_used)] // テストコードではunwrapを許可
#![allow(missing_docs)] // テストコードではdocコメント不要

use assert_cmd::cargo_bin_cmd;
use predicates::prelude::predicate;

#[test]
fn test_cli_with_custom_name() {
    let mut cmd = cargo_bin_cmd!("brust");
    cmd.arg("--name")
        .arg("Alice")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hi, Alice, new world!!"));
}

#[test]
fn test_cli_with_short_flag() {
    let mut cmd = cargo_bin_cmd!("brust");
    cmd.arg("-n")
        .arg("Bob")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hi, Bob, new world!!"));
}

#[test]
fn test_cli_version_flag() {
    let mut cmd = cargo_bin_cmd!("brust");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("brust version"));
}

#[test]
fn test_cli_version_short_flag() {
    let mut cmd = cargo_bin_cmd!("brust");
    cmd.arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::contains("brust version"));
}

#[test]
fn test_cli_with_gender_man() {
    let mut cmd = cargo_bin_cmd!("brust");
    cmd.arg("--name")
        .arg("John")
        .arg("--gender")
        .arg("man")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hi, Mr. John, new world!!"));
}

#[test]
fn test_cli_with_gender_woman() {
    let mut cmd = cargo_bin_cmd!("brust");
    cmd.arg("--name")
        .arg("Alice")
        .arg("--gender")
        .arg("woman")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hi, Ms. Alice, new world!!"));
}

#[test]
fn test_cli_with_gender_short_flag() {
    let mut cmd = cargo_bin_cmd!("brust");
    cmd.arg("-n")
        .arg("Bob")
        .arg("-g")
        .arg("man")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hi, Mr. Bob, new world!!"));
}

#[test]
fn test_cli_with_invalid_gender() {
    let mut cmd = cargo_bin_cmd!("brust");
    cmd.arg("--name")
        .arg("Charlie")
        .arg("--gender")
        .arg("other")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Hi, Charlie (invalid gender: other), new world!!",
        ));
}

#[test]
fn test_cli_without_gender() {
    let mut cmd = cargo_bin_cmd!("brust");
    cmd.arg("--name")
        .arg("Dave")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hi, Dave, new world!!"));
}
