#![allow(missing_docs)]

use std::process::Command;

fn main() {
    #[allow(clippy::expect_used)] // ビルドスクリプトでは必要
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .expect("Failed to retrieve Git commit hash.");
    #[allow(clippy::expect_used)] // ビルドスクリプトでは必要
    let git_hash =
        String::from_utf8(output.stdout).expect("Failed to convert Git commit hash to string.");

    println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
}
