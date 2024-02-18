use std::{
    fs::{copy, create_dir, read_to_string, write},
    path::Path,
};

use helpers::*;
use snapbox::{
    assert_eq,
    cmd::{cargo_bin, Command},
    Data,
};

mod helpers;

/// Run a `PreRelease` then `Release` for a repo not configured for GitHub.
///
/// # Expected
///
/// Version should be bumped, and a new tag should be added to the repo.
#[test]
fn github_release() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/github_release");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");

    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Act. Cannot run real release without integration testing GitHub.
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
}

/// Verify that Release will operate on all defined packages independently
#[test]
fn multiple_packages() {
    // Arrange.
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let data_path = Path::new("tests/github_release/multiple_packages");

    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "first/v1.2.3");
    tag(temp_path, "second/v0.4.6");
    commit(temp_path, "feat!: New breaking feature");

    copy_dir_contents(&data_path.join("source"), temp_path);

    // Act.
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(&data_path.join("dry_run_output.txt"), None));
}

#[test]
fn separate_prepare_and_release_workflows() {
    // Arrange a package that is ready to release, but hasn't been released yet
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/github_release/separate_prepare_and_release_workflows");
    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");
    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }
    Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Run the actual release (but dry-run because don't test GitHub)
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
}

#[test]
fn release_assets() {
    // Arrange a package that's ready to release with some artifacts
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/github_release/release_assets");
    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");
    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }
    Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .current_dir(temp_dir.path())
        .assert()
        .success();
    let assets_dir = temp_path.join("assets");
    create_dir(&assets_dir).unwrap();
    write(assets_dir.join("first"), "first").unwrap();
    write(assets_dir.join("second"), "second").unwrap();

    // Actual release
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
}

#[test]
fn auto_generate_release_notes() {
    // Arrange a package that is ready to release, but hasn't been released yet
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/github_release/separate_prepare_and_release_workflows");
    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    commit(temp_path, "feat: New feature");
    for file in ["knope.toml", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }
    Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    // Run the actual release (but dry-run because don't test GitHub)
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("auto_generate_dry_run_output.txt"),
            None,
        ));
}

#[test]
fn no_previous_tag() {
    // Arrange a package that is ready to release, but hasn't been released yet
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/github_release/no_previous_tag");
    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }

    // Run the actual release (but dry-run because don't test GitHub)
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
}

#[test]
fn version_go_mod() {
    // Arrange a package that is ready to release, but hasn't been released yet
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path();
    let source_path = Path::new("tests/github_release/version_go_mod");
    init(temp_path);
    commit(temp_path, "feat: Existing feature");
    tag(temp_path, "v1.0.0");
    tag(temp_path, "go/v1.0.0");
    commit(temp_path, "feat: New feature");
    for file in ["knope.toml", "CHANGELOG.md", "Cargo.toml"] {
        copy(source_path.join(file), temp_path.join(file)).unwrap();
    }
    create_dir(temp_path.join("go")).unwrap();
    write(temp_path.join("go/go.mod"), "module github.com/owner/repo").unwrap();
    Command::new(cargo_bin!("knope"))
        .arg("prepare-release")
        .current_dir(temp_dir.path())
        .assert()
        .success();
    commit(temp_path, "chore: Prepare release");
    assert_eq(
        Data::read_from(&source_path.join("expected_go.mod"), None),
        read_to_string(temp_path.join("go/go.mod")).unwrap(),
    );

    // Run the actual release (but dry-run because don't test GitHub)
    let dry_run_assert = Command::new(cargo_bin!("knope"))
        .arg("release")
        .arg("--dry-run")
        .current_dir(temp_dir.path())
        .assert();

    // Assert.
    dry_run_assert
        .success()
        .with_assert(assert())
        .stdout_matches(Data::read_from(
            &source_path.join("dry_run_output.txt"),
            None,
        ));
}
