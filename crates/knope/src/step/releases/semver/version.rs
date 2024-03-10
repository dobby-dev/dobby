use std::{cmp::Ordering, fmt::Display, str::FromStr};

use miette::Diagnostic;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Version {
    Stable(StableVersion),
    Pre(PreVersion),
}

impl Version {
    pub(crate) const fn stable_component(&self) -> StableVersion {
        match self {
            Self::Stable(stable) => *stable,
            Self::Pre(pre) => pre.stable_component,
        }
    }

    pub(crate) const fn is_prerelease(&self) -> bool {
        matches!(self, Version::Pre(_))
    }
}

impl Version {
    pub(crate) fn new(major: u64, minor: u64, patch: u64, pre: Option<Prerelease>) -> Self {
        let stable = StableVersion {
            major,
            minor,
            patch,
        };
        match pre {
            Some(pre) => Self::Pre(PreVersion {
                stable_component: stable,
                pre_component: pre,
            }),
            None => Self::Stable(stable),
        }
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::new(0, 0, 0, None)
    }
}

impl From<StableVersion> for Version {
    fn from(stable: StableVersion) -> Self {
        Self::Stable(stable)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct StableVersion {
    pub(crate) major: u64,
    pub(crate) minor: u64,
    pub(crate) patch: u64,
}

impl StableVersion {
    pub(crate) const fn increment_major(self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
            patch: 0,
        }
    }

    pub(crate) const fn increment_minor(self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
        }
    }

    pub(crate) const fn increment_patch(self) -> Self {
        Self {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
        }
    }
}

impl Ord for StableVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
    }
}

impl PartialOrd for StableVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for StableVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{major}.{minor}.{patch}",
            major = self.major,
            minor = self.minor,
            patch = self.patch
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PreVersion {
    pub(crate) stable_component: StableVersion,
    pub(crate) pre_component: Prerelease,
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.stable_component().cmp(&other.stable_component()) {
            Ordering::Equal => match (self, other) {
                (Self::Stable(_), Self::Stable(_)) => Ordering::Equal,
                (Self::Stable(_), Self::Pre(_)) => Ordering::Greater,
                (Self::Pre(_), Self::Stable(_)) => Ordering::Less,
                (Self::Pre(pre), Self::Pre(other_pre)) => {
                    pre.pre_component.cmp(&other_pre.pre_component)
                }
            },
            ordering => ordering,
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (version, pre) = s
            .split_once('-')
            .map_or((s, None), |(version, pre)| (version, Some(pre)));
        let version_parts: [u64; 3] = version
            .split('.')
            .map(|part| part.parse::<u64>().map_err(|err| Error(err.to_string())))
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| Error("Version must have exactly 3 parts".to_string()))?;
        let stable = StableVersion {
            major: version_parts[0],
            minor: version_parts[1],
            patch: version_parts[2],
        };
        if let Some(pre) = pre {
            Ok(Self::Pre(PreVersion {
                stable_component: stable,
                pre_component: Prerelease::from_str(pre)?,
            }))
        } else {
            Ok(Self::Stable(stable))
        }
    }
}

#[derive(Debug, Diagnostic, thiserror::Error)]
#[error("Found invalid semantic version {0}")]
#[diagnostic(
    code(version),
    help("The version must be a valid Semantic Version"),
    url("https://knope.tech/reference/concepts/semantic-versioning")
)]
pub(crate) struct Error(String);

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stable(StableVersion {
                major,
                minor,
                patch,
            }) => write!(f, "{major}.{minor}.{patch}"),
            Self::Pre(PreVersion {
                stable_component:
                    StableVersion {
                        major,
                        minor,
                        patch,
                    },
                pre_component,
            }) => write!(f, "{major}.{minor}.{patch}-{pre_component}",),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Prerelease {
    pub(crate) label: Label,
    pub(crate) version: u64,
}

impl Display for Prerelease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.label, self.version)
    }
}

impl FromStr for Prerelease {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (label, version) = s
            .split_once('.')
            .ok_or_else(|| Error("Invalid prerelease".to_string()))?;
        Ok(Self {
            label: Label(String::from(label)),
            version: version
                .parse::<u64>()
                .map_err(|err| Error(err.to_string()))?,
        })
    }
}

impl Ord for Prerelease {
    fn cmp(&self, other: &Self) -> Ordering {
        self.label
            .cmp(&other.label)
            .then(self.version.cmp(&other.version))
    }
}

impl PartialOrd for Prerelease {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Prerelease {
    pub(crate) fn new(label: Label, version: u64) -> Self {
        Self { label, version }
    }
}

/// The label component of a Prerelease (e.g., "alpha" in "1.0.0-alpha.1").
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[repr(transparent)]
pub(crate) struct Label(pub(crate) String);

impl Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Label {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
