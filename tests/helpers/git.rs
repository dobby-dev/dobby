use std::{path::Path, process::Command};

use itertools::Itertools;
use log::debug;

/// Create a Git repo in `path` with some fake config.
pub fn init(path: &Path) {
    let output = Command::new("git")
        .arg("init")
        .arg("--initial-branch=main")
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

/// Create and switch to a new branch
pub fn create_branch(path: &Path, name: &str) {
    let output = Command::new("git")
        .arg("switch")
        .arg("-c")
        .arg(name)
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    debug!("{}", String::from_utf8_lossy(&output.stdout));
}

/// Switch to an existing branch
pub fn switch_branch(path: &Path, name: &str) {
    let output = Command::new("git")
        .arg("switch")
        .arg(name)
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    debug!("{}", String::from_utf8_lossy(&output.stdout));
}

/// Merge a branch into the current branch
pub fn merge_branch(path: &Path, name: &str) {
    let output = Command::new("git")
        .arg("merge")
        .arg("--no-ff")
        .arg(name)
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    debug!("{}", String::from_utf8_lossy(&output.stdout));
}

/// Get the current tag, panicking if there is no tag.
pub fn describe(path: &Path, pattern: Option<&str>) -> String {
    let mut cmd = Command::new("git");
    cmd.arg("describe").arg("--tags");
    if let Some(pattern) = pattern {
        cmd.arg("--match").arg(pattern);
    }

    let output = cmd.current_dir(path).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Add all files to git
pub fn add_all(path: &Path) {
    let output = Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// See which files, if any, are dirty in a Git repo
pub fn status(path: &Path) -> Vec<String> {
    let output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(String::from)
        .sorted()
        .collect()
}
