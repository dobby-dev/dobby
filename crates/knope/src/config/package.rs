use std::{fmt, fmt::Display, ops::Range, path::PathBuf, str::FromStr};

use ::toml::{from_str, Value};
use git_conventional::FooterToken;
use itertools::Itertools;
use knope_versioning::{cargo, VersionedFilePath};
use miette::Diagnostic;
use relative_path::{RelativePath, RelativePathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::toml;
use crate::{
    fs,
    fs::read_to_string,
    step::releases::{
        changelog,
        package::{Asset, ChangelogSectionSource},
        ChangeType, PackageName,
    },
};

/// Represents a single package in `knope.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Package {
    pub(crate) name: Option<PackageName>,
    /// The files which define the current version of the package.
    pub(crate) versioned_files: Vec<VersionedFilePath>,
    /// The path to the `CHANGELOG.md` file (if any) to be updated when running [`Step::PrepareRelease`].
    pub(crate) changelog: Option<RelativePathBuf>,
    /// Optional scopes that can be used to filter commits when running [`Step::PrepareRelease`].
    pub(crate) scopes: Option<Vec<String>>,
    /// Extra sections that should be added to the changelog from custom footers in commit messages
    /// or change set types.
    pub(crate) extra_changelog_sections: Vec<ChangelogSection>,
    pub(crate) assets: Option<Vec<Asset>>,
    pub(crate) ignore_go_major_versioning: bool,
}

impl Package {
    pub(crate) fn find_in_working_dir() -> Result<Vec<Self>> {
        let packages = Self::cargo_workspace_members()?;

        if !packages.is_empty() {
            return Ok(packages);
        }

        let default_changelog_path = RelativePathBuf::from("CHANGELOG.md");
        let changelog = default_changelog_path
            .to_path("")
            .exists()
            .then_some(default_changelog_path);

        let versioned_files = VersionedFilePath::defaults()
            .into_iter()
            .filter_map(|file_name| {
                let path = file_name.as_path();
                if path.to_path("").exists() {
                    Some(file_name)
                } else {
                    None
                }
            })
            .collect_vec();
        if versioned_files.is_empty() {
            Ok(vec![])
        } else {
            Ok(vec![Self {
                versioned_files,
                changelog,
                ..Self::default()
            }])
        }
    }

    fn cargo_workspace_members() -> std::result::Result<Vec<Self>, CargoWorkspaceError> {
        let path = RelativePath::new("Cargo.toml");
        let Ok(contents) = read_to_string(path.as_str()) else {
            return Ok(Vec::new());
        };
        let cargo_toml = Value::from_str(&contents)
            .map_err(|err| CargoWorkspaceError::Toml(err, path.into()))?;
        let workspace_path = path
            .parent()
            .ok_or_else(|| CargoWorkspaceError::Parent(path.into()))?;
        let Some(members) = cargo_toml
            .get("workspace")
            .and_then(|workspace| workspace.as_table()?.get("members")?.as_array())
        else {
            return Ok(Vec::new());
        };
        members
            .iter()
            .map(|member_val| {
                let member = member_val.as_str().ok_or(CargoWorkspaceError::Members)?;
                let member_path =
                    VersionedFilePath::new(workspace_path.join(member).join("Cargo.toml"))?;
                let member_contents = read_to_string(member_path.as_path().to_path("."))?;
                from_str::<cargo::Toml>(&member_contents)
                    .map_err(|err| CargoWorkspaceError::Toml(err, member_path.as_path()))
                    .map(|cargo| Self {
                        name: Some(PackageName::from(cargo.package.name.clone())),
                        versioned_files: vec![member_path],
                        scopes: Some(vec![cargo.package.name]),
                        ..Self::default()
                    })
            })
            .collect()
    }

    pub(crate) fn from_toml(
        name: Option<PackageName>,
        package: toml::Package,
        source_code: &str,
    ) -> std::result::Result<Self, VersionedFileError> {
        let toml::Package {
            versioned_files,
            changelog,
            scopes,
            extra_changelog_sections,
            assets,
            ignore_go_major_versioning,
        } = package;
        let versioned_files = versioned_files
            .into_iter()
            .map(|spanned| {
                let span = spanned.span();
                VersionedFilePath::new(spanned.into_inner())
                    .map_err(|source| VersionedFileError::Unknown {
                        file_name: source.path.file_name().unwrap_or_default().to_string(),
                        span: span.clone(),
                        source_code: source_code.to_string(),
                    })
                    .and_then(|path| {
                        let pathbuf = path.to_pathbuf();
                        if pathbuf.exists() {
                            Ok(path)
                        } else {
                            Err(VersionedFileError::Missing {
                                path: pathbuf,
                                span,
                                source_code: source_code.to_string(),
                            })
                        }
                    })
            })
            .try_collect()?;
        Ok(Self {
            name,
            versioned_files,
            changelog,
            scopes,
            extra_changelog_sections,
            assets,
            ignore_go_major_versioning,
        })
    }
}

#[derive(Debug, Diagnostic, Error)]
pub enum VersionedFileError {
    #[error("Unknown file name {file_name}")]
    #[diagnostic(
        code(config::unknown_versioned_file),
        help("Knope relies on the name of the file to determine its type."),
        url("https://knope.tech/reference/config-file/packages#versioned_files")
    )]
    Unknown {
        file_name: String,
        #[source_code]
        source_code: String,
        #[label("Declared here")]
        span: Range<usize>,
    },
    #[error("File {path} does not exist")]
    #[diagnostic(
        code(config::missing_versioned_file),
        help("Make sure the file exists and is accessible.")
    )]
    Missing {
        path: PathBuf,
        #[source_code]
        source_code: String,
        #[label("Declared here")]
        span: Range<usize>,
    },
}

#[derive(Debug, Diagnostic, thiserror::Error)]
pub(crate) enum CargoWorkspaceError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Fs(#[from] fs::Error),
    #[error("Could not parse TOML in {1}: {0}")]
    #[diagnostic(code(workspace::toml))]
    Toml(::toml::de::Error, RelativePathBuf),
    #[error("Could not get parent directory of Cargo.toml file: {0}")]
    #[diagnostic(code(workspace::parent))]
    Parent(RelativePathBuf),
    #[error("The Cargo workspace members array should contain only strings")]
    #[diagnostic(code(workspace::members))]
    Members,
    #[error(transparent)]
    #[diagnostic(transparent)]
    UnknownFile(#[from] knope_versioning::UnknownFile),
}

#[derive(Debug, Diagnostic, Error)]
pub(crate) enum Error {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Changelog(#[from] changelog::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    CargoWorkspace(#[from] CargoWorkspaceError),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ChangelogSection {
    pub(crate) name: ChangeLogSectionName,
    #[serde(default)]
    pub(crate) footers: Vec<CommitFooter>,
    #[serde(default)]
    pub(crate) types: Vec<CustomChangeType>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct CommitFooter(String);

impl Display for CommitFooter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<FooterToken<'_>> for CommitFooter {
    fn from(token: FooterToken<'_>) -> Self {
        Self(token.to_string())
    }
}

impl From<&str> for CommitFooter {
    fn from(token: &str) -> Self {
        Self(token.into())
    }
}

impl From<CommitFooter> for ChangeType {
    fn from(footer: CommitFooter) -> Self {
        Self::Custom(ChangelogSectionSource::CommitFooter(footer))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct CustomChangeType(String);

impl Display for CustomChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for CustomChangeType {
    fn from(token: String) -> Self {
        Self(token)
    }
}

impl From<&str> for CustomChangeType {
    fn from(token: &str) -> Self {
        Self(token.into())
    }
}

impl From<CustomChangeType> for String {
    fn from(custom: CustomChangeType) -> Self {
        custom.0
    }
}

impl From<CustomChangeType> for ChangeType {
    fn from(custom: CustomChangeType) -> Self {
        changesets::ChangeType::from(String::from(custom)).into()
    }
}

impl From<changesets::ChangeType> for ChangeType {
    fn from(change_type: changesets::ChangeType) -> Self {
        match change_type {
            changesets::ChangeType::Major => Self::Breaking,
            changesets::ChangeType::Minor => Self::Feature,
            changesets::ChangeType::Patch => Self::Fix,
            changesets::ChangeType::Custom(custom) => {
                Self::Custom(ChangelogSectionSource::CustomChangeType(custom.into()))
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct ChangeLogSectionName(String);

impl Display for ChangeLogSectionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ChangeLogSectionName {
    fn from(token: &str) -> Self {
        Self(token.into())
    }
}

impl AsRef<str> for ChangeLogSectionName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
