use semver::{Prerelease, Version};
use serde::{Deserialize, Serialize};

use crate::releases::git::get_current_versions_from_tag;
use crate::releases::package::{Package, VersionedFile};
use crate::releases::CurrentVersions;
use crate::step::StepError;
use crate::{state, RunType};

use super::PackageConfig;

/// The various rules that can be used when bumping the current version of a project via
/// [`crate::step::Step::BumpVersion`].
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "rule")]
pub(crate) enum Rule {
    Major,
    Minor,
    Patch,
    Pre {
        label: String,
        #[serde(skip)]
        stable_rule: ConventionalRule,
    },
    Release,
}

impl From<ConventionalRule> for Rule {
    fn from(conventional_rule: ConventionalRule) -> Self {
        match conventional_rule {
            ConventionalRule::Major => Rule::Major,
            ConventionalRule::Minor => Rule::Minor,
            ConventionalRule::Patch => Rule::Patch,
        }
    }
}

/// The rules that can be derived from Conventional Commits.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ConventionalRule {
    Major,
    Minor,
    Patch,
}

impl Default for ConventionalRule {
    fn default() -> Self {
        ConventionalRule::Patch
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct PackageVersion {
    /// The current versions for the package
    pub(crate) version: CurrentVersions,
    /// The package from which the version was derived (and the package that should be bumped).
    pub(crate) package: Package,
}

impl PackageVersion {
    pub(crate) fn latest_version(&self) -> &Version {
        self.version.latest()
    }
}

pub(super) fn bump_version(
    rule: Rule,
    dry_run: bool,
    packages: &[PackageConfig],
) -> Result<Version, StepError> {
    let mut package_version = get_version(packages)?;
    package_version.version = bump(package_version.version, rule)?;
    set_version(package_version, dry_run)
}

pub(crate) fn bump_version_and_update_state(
    run_type: RunType,
    rule: Rule,
) -> Result<RunType, StepError> {
    match run_type {
        RunType::DryRun {
            mut state,
            mut stdout,
        } => {
            let version = bump_version(rule, true, &state.packages)?;
            writeln!(stdout, "Would bump version to {}", version)?;
            state.release = state::Release::Bumped(version);
            Ok(RunType::DryRun { state, stdout })
        }
        RunType::Real(mut state) => {
            let version = bump_version(rule, false, &state.packages)?;
            state.release = state::Release::Bumped(version);
            Ok(RunType::Real(state))
        }
    }
}

pub(crate) fn get_version(packages: &[PackageConfig]) -> Result<PackageVersion, StepError> {
    if packages.is_empty() {
        return Err(StepError::no_defined_packages_with_help());
    }
    if packages.len() > 1 {
        return Err(StepError::TooManyPackages);
    }
    let package_config = &packages[0];
    let package = Package::try_from(package_config.clone())?;
    let stable_version = package
        .versioned_files
        .iter()
        .map(VersionedFile::get_version)
        .map(|result| {
            result.and_then(|version_string| {
                Version::parse(&version_string)
                    .map_err(|_| StepError::InvalidSemanticVersion(version_string))
            })
        })
        .reduce(|accumulator, version| match (version, accumulator) {
            (Ok(version), Ok(accumulator)) => {
                if version == accumulator {
                    Ok(accumulator)
                } else {
                    Err(StepError::InconsistentVersions(
                        version.to_string(),
                        accumulator.to_string(),
                    ))
                }
            }
            (_, Err(err)) | (Err(err), _) => Err(err),
        })
        .transpose()?;

    let version = match stable_version {
        None => get_current_versions_from_tag()?.unwrap_or_default(),
        Some(stable) if stable.pre.is_empty() => CurrentVersions {
            stable,
            prerelease: None,
        },
        Some(pre) => {
            let stable = get_current_versions_from_tag()?.map_or_else(
                || Version::new(0, 0, 0),
                |current_versions| current_versions.stable,
            );
            CurrentVersions {
                stable,
                prerelease: Some(pre),
            }
        }
    };

    Ok(PackageVersion { version, package })
}

/// Consumes a [`PackageVersion`], writing it back to the file it came from. Returns the new version
/// that was written.
fn set_version(package_version: PackageVersion, dry_run: bool) -> Result<Version, StepError> {
    let PackageVersion {
        version: CurrentVersions { stable, prerelease },
        package,
    } = package_version;
    let version = prerelease.unwrap_or(stable);
    if dry_run {
        return Ok(version);
    }
    for versioned_file in package.versioned_files {
        versioned_file.set_version(&version)?;
    }
    Ok(version)
}

/// Apply a Rule to a [`PackageVersion`], incrementing & resetting the correct components.
///
/// ### Versions 0.x
///
/// Versions with major component 0 have special meaning in Semantic Versioning and therefore have
/// different behavior:
/// 1. [`Rule::Major`] will bump the minor component.
/// 2. [`Rule::Minor`] will bump the patch component.
fn bump(mut version: CurrentVersions, rule: Rule) -> Result<CurrentVersions, StepError> {
    let stable = &mut version.stable;
    let is_0 = stable.major == 0;
    let prerelease = version.prerelease.take();
    match (rule, is_0) {
        (Rule::Major, false) => {
            stable.major += 1;
            stable.minor = 0;
            stable.patch = 0;
            stable.pre = Prerelease::EMPTY;
            Ok(version)
        }
        (Rule::Minor, false) | (Rule::Major, true) => {
            stable.minor += 1;
            stable.patch = 0;
            stable.pre = Prerelease::EMPTY;
            Ok(version)
        }
        (Rule::Patch, _) | (Rule::Minor, true) => {
            stable.patch += 1;
            stable.pre = Prerelease::EMPTY;
            Ok(version)
        }
        (Rule::Release, _) => {
            let mut prerelease = prerelease.ok_or_else(|| {
                StepError::InvalidPreReleaseVersion(
                    "No prerelease version found, but a Release rule was requested".to_string(),
                )
            })?;
            prerelease.pre = Prerelease::EMPTY;
            *stable = prerelease;
            Ok(version)
        }
        (Rule::Pre { label, stable_rule }, _) => bump_pre(version, prerelease, &label, stable_rule),
    }
}

#[cfg(test)]
mod test_bump {
    use super::*;

    use rstest::rstest;

    #[test]
    fn major() {
        let stable = Version::new(1, 2, 3);
        let version = bump(
            CurrentVersions {
                stable,
                prerelease: None,
            },
            Rule::Major,
        )
        .unwrap();

        assert_eq!(version.stable, Version::new(2, 0, 0));
    }

    #[test]
    fn major_0() {
        let stable = Version::new(0, 1, 2);
        let version = bump(
            CurrentVersions {
                stable,
                prerelease: None,
            },
            Rule::Major,
        )
        .unwrap();

        assert_eq!(version.stable, Version::new(0, 2, 0));
    }

    #[rstest]
    #[case("1.2.4-rc.0")]
    #[case("1.3.0-rc.0")]
    #[case("2.0.0-rc.0")]
    fn major_after_pre(#[case] pre_version: &str) {
        let stable = Version::new(1, 2, 3);
        let version = bump(
            CurrentVersions {
                stable,
                prerelease: Some(Version::parse(pre_version).unwrap()),
            },
            Rule::Major,
        )
        .unwrap();

        assert_eq!(version.stable, Version::new(2, 0, 0));
        assert!(version.prerelease.is_none());
    }

    #[test]
    fn minor() {
        let stable = Version::new(1, 2, 3);
        let version = bump(
            CurrentVersions {
                stable,
                prerelease: None,
            },
            Rule::Minor,
        )
        .unwrap();

        assert_eq!(version.stable, Version::new(1, 3, 0));
    }

    #[test]
    fn minor_0() {
        let stable = Version::new(0, 1, 2);
        let version = bump(
            CurrentVersions {
                stable,
                prerelease: None,
            },
            Rule::Minor,
        )
        .unwrap();

        assert_eq!(version.stable, Version::new(0, 1, 3));
    }

    #[rstest]
    #[case("1.2.4-rc.0")]
    #[case("1.3.0-rc.0")]
    fn minor_after_pre(#[case] pre_version: &str) {
        let stable = Version::new(1, 2, 3);
        let version = bump(
            CurrentVersions {
                stable,
                prerelease: Some(Version::parse(pre_version).unwrap()),
            },
            Rule::Minor,
        )
        .unwrap();

        assert_eq!(version.stable, Version::new(1, 3, 0));
        assert!(version.prerelease.is_none());
    }

    #[test]
    fn patch() {
        let stable = Version::new(1, 2, 3);
        let version = bump(
            CurrentVersions {
                stable,
                prerelease: None,
            },
            Rule::Patch,
        )
        .unwrap();

        assert_eq!(version.stable, Version::new(1, 2, 4));
    }

    #[test]
    fn patch_0() {
        let stable = Version::new(0, 1, 0);
        let version = bump(
            CurrentVersions {
                stable,
                prerelease: None,
            },
            Rule::Patch,
        )
        .unwrap();

        assert_eq!(version.stable, Version::new(0, 1, 1));
    }

    #[test]
    fn patch_after_pre() {
        let stable = Version::new(1, 2, 3);
        let version = bump(
            CurrentVersions {
                stable,
                prerelease: Some(Version::parse("1.2.4-rc.0").unwrap()),
            },
            Rule::Patch,
        )
        .unwrap();

        assert_eq!(version.stable, Version::new(1, 2, 4));
        assert!(version.prerelease.is_none());
    }

    #[test]
    fn pre() {
        let stable = Version::new(1, 2, 3);
        let new = bump(
            CurrentVersions {
                stable: stable.clone(),
                prerelease: None,
            },
            Rule::Pre {
                label: String::from("rc"),
                stable_rule: ConventionalRule::Minor,
            },
        )
        .unwrap();

        assert_eq!(new.prerelease, Some(Version::parse("1.3.0-rc.0").unwrap()));
        assert_eq!(new.stable, stable);
    }

    #[test]
    fn pre_after_same_pre() {
        let stable = Version::new(1, 2, 3);
        let prerelease = Some(Version::parse("1.3.0-rc.0").unwrap());
        let new = bump(
            CurrentVersions {
                stable: stable.clone(),
                prerelease,
            },
            Rule::Pre {
                label: String::from("rc"),
                stable_rule: ConventionalRule::Minor,
            },
        )
        .unwrap();

        assert_eq!(new.prerelease, Some(Version::parse("1.3.0-rc.1").unwrap()));
        assert_eq!(new.stable, stable);
    }

    #[test]
    fn pre_after_different_pre_version() {
        let stable = Version::new(1, 2, 3);
        let prerelease = Some(Version::parse("1.2.4-rc.0").unwrap());
        let new = bump(
            CurrentVersions {
                stable: stable.clone(),
                prerelease,
            },
            Rule::Pre {
                label: String::from("rc"),
                stable_rule: ConventionalRule::Minor,
            },
        )
        .unwrap();

        assert_eq!(new.prerelease, Some(Version::parse("1.3.0-rc.0").unwrap()));
        assert_eq!(new.stable, stable);
    }

    #[test]
    fn pre_after_different_pre_label() {
        let stable = Version::new(1, 2, 3);
        let prerelease = Some(Version::parse("1.3.0-beta.0").unwrap());
        let new = bump(
            CurrentVersions {
                stable: stable.clone(),
                prerelease,
            },
            Rule::Pre {
                label: String::from("rc"),
                stable_rule: ConventionalRule::Minor,
            },
        )
        .unwrap();

        assert_eq!(new.prerelease, Some(Version::parse("1.3.0-rc.0").unwrap()));
        assert_eq!(new.stable, stable);
    }

    #[test]
    fn release() {
        let version = bump(
            CurrentVersions {
                stable: Version::new(1, 2, 3),
                prerelease: Some(Version::parse("1.2.3-rc.0").unwrap()),
            },
            Rule::Release,
        )
        .unwrap();

        assert_eq!(version.stable, Version::new(1, 2, 3));
        assert!(version.prerelease.is_none());
    }
}

/// Bumps the pre-release component of a [`Version`].
///
/// If the existing [`Version`] has no pre-release,
/// `semantic_rule` will be used to bump to primary components before the
/// pre-release component is added.
///
/// # Errors
///
/// Can fail if there is an existing pre-release component that can't be incremented.
fn bump_pre(
    stable_only: CurrentVersions,
    prerelease: Option<Version>,
    label: &str,
    stable_rule: ConventionalRule,
) -> Result<CurrentVersions, StepError> {
    let stable = stable_only.stable.clone();
    let next_stable = bump(stable_only, stable_rule.into())?.stable;
    let prerelease_version = prerelease
        .and_then(|prerelease| {
            if prerelease.major != next_stable.major
                || prerelease.minor != next_stable.minor
                || prerelease.patch != next_stable.patch
            {
                return None;
            }
            let pre_string = prerelease.pre.as_str();
            let parts = pre_string.split('.').collect::<Vec<_>>();
            if parts.len() != 2 || parts[0] != label {
                return None;
            }
            let pre_version = parts[1].parse::<u16>().ok()?;
            Some(format!("{}.{}", label, pre_version + 1))
        })
        .unwrap_or_else(|| format!("{}.0", label));

    let mut next_prerelease = next_stable;
    next_prerelease.pre = Prerelease::new(&prerelease_version)
        .map_err(|_| StepError::InvalidPreReleaseVersion(prerelease_version))?;
    Ok(CurrentVersions {
        stable,
        prerelease: Some(next_prerelease),
    })
}
