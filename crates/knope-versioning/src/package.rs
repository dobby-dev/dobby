use itertools::Itertools;
#[cfg(feature = "miette")]
use miette::Diagnostic;
use thiserror::Error;

use crate::{
    action::Action,
    go_mod::GoVersioning,
    versioned_file::{SetError, VersionedFile},
    Version,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
    versioned_files: Vec<VersionedFile>,
}

impl Package {
    /// Try and combine a bunch of versioned file into one logical package.
    ///
    /// # Errors
    ///
    /// There must be at least one versioned file and all files must have the same version.
    pub fn new(versioned_files: Vec<VersionedFile>) -> Result<Self, NewError> {
        if let Some(first) = versioned_files.first() {
            if let Some(conflict) = versioned_files
                .iter()
                .find(|f| f.version() != first.version())
            {
                return Err(NewError::InconsistentVersions(
                    Box::new(first.clone()),
                    Box::new(conflict.clone()),
                ));
            }
        } else {
            return Err(NewError::NoPackages);
        }
        Ok(Self { versioned_files })
    }

    #[must_use]
    pub fn versioned_files(&self) -> &[VersionedFile] {
        &self.versioned_files
    }

    #[must_use]
    #[allow(clippy::indexing_slicing)] // Construction check guarantees there is at least one versioned file
    pub fn get_version(&self) -> &Version {
        self.versioned_files[0].version()
    }

    /// Returns the actions that must be taken to set this package to the new version.
    ///
    /// # Errors
    ///
    /// If the file is a `go.mod`, there are rules about what versions are allowed.
    ///
    /// If serialization of some sort fails, which is a bug, then this will return an error.
    pub fn set_version(
        self,
        new_version: &Version,
        go_versioning: GoVersioning,
    ) -> Result<Vec<Action>, SetError> {
        self.versioned_files
            .into_iter()
            .map(|f| f.set_version(new_version, go_versioning))
            .process_results(|iter| iter.flatten().collect())
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(Diagnostic))]
pub enum NewError {
    #[error("Found inconsistent versions in package: {} had {} and {} had {}", .0.path(), .0.version(), .1.path(), .1.version())]
    #[cfg_attr(
        feature = "miette",
        diagnostic(
            code = "knope_versioning::inconsistent_versions",
            url = "https://knope.tech/reference/concepts/package/#version",
            help = "All files in a package must have the same version"
        )
    )]
    InconsistentVersions(Box<VersionedFile>, Box<VersionedFile>),
    #[error("Packages must have at least one versioned file")]
    NoPackages,
}
