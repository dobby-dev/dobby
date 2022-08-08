use ::semver::Version;
pub(crate) use conventional_commits::update_project_from_conventional_commits as prepare_release;

use crate::state::Release::Prepared;
use crate::step::StepError;
use crate::RunType;

pub(crate) use self::git::get_current_versions_from_tag;
pub(crate) use self::package::{find_packages, suggested_package_toml, PackageConfig};
pub(crate) use self::semver::bump_version_and_update_state as bump_version;
pub(crate) use self::semver::{get_version, Rule};

mod cargo;
mod changelog;
mod conventional_commits;
mod git;
mod github;
mod go;
mod package;
mod package_json;
mod pyproject;
mod semver;

#[derive(Clone, Debug)]
pub(crate) struct Release {
    pub(crate) version: Version,
    pub(crate) changelog: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CurrentVersions {
    pub(crate) stable: Version,
    pub(crate) prerelease: Option<Version>,
}

impl CurrentVersions {
    pub(crate) fn latest(&self) -> &Version {
        self.prerelease.as_ref().unwrap_or(&self.stable)
    }

    pub(crate) fn into_latest(self) -> Version {
        self.prerelease.unwrap_or(self.stable)
    }
}

impl Default for CurrentVersions {
    fn default() -> Self {
        Self {
            stable: Version::new(0, 0, 0),
            prerelease: None,
        }
    }
}

/// Create a release for the package.
///
/// If GitHub config is present, this creates a GitHub release. Otherwise, it tags the Git repo.
pub(crate) fn release(run_type: RunType) -> Result<RunType, StepError> {
    let (state, dry_run_stdout) = run_type.decompose();

    let release = match state.release.clone() {
        Prepared(release) => release,
        _ => return Err(StepError::ReleaseNotPrepared),
    };

    let github_config = state.github_config.clone();
    if let Some(github_config) = github_config {
        github::release(state, dry_run_stdout, &github_config, &release)
    } else {
        git::release(state, dry_run_stdout, &release)
    }
}
