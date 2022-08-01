#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

/// Create a Git repo in `path` with some fake config.
pub fn init(path: &Path) {
    let output = Command::new("git")
        .arg("init")
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Configure fake Git user.
    let output = Command::new("git")
        .arg("config")
        .arg("user.email")
        .arg("fake@knope.dev")
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("Fake knope")
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Add a Git remote to the repo at `path`.
pub fn add_remote(path: &Path, remote: &str) {
    let output = Command::new("git")
        .arg("remote")
        .arg("add")
        .arg("origin")
        .arg(remote)
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Create a commit with `message` in the Git repo which exists in `path`.
pub fn commit(path: &Path, message: &str) {
    let output = Command::new("git")
        .arg("commit")
        .arg("--allow-empty")
        .arg("-m")
        .arg(message)
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Create a tag with `label` in the Git repo which exists in `path`.
pub fn tag(path: &Path, label: &str) {
    let output = Command::new("git")
        .arg("tag")
        .arg(label)
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Get the current tag, panicking if there is no tag.
pub fn describe(path: &Path) -> String {
    let output = Command::new("git")
        .arg("describe")
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
