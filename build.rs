#![allow(missing_docs)]

use std::process::Command;

fn main() {
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| String::from("unknown"), |s| s.trim().to_owned());

    println!("cargo:rustc-env=GIT_HASH={git_hash}");
}
