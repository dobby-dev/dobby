use std::{fmt::Debug, io::sink};

use itertools::Itertools;
use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    state::RunType,
    step::{Step, StepError},
    State,
};

/// A workflow is basically the state machine to run for a single execution of knope.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Workflow {
    /// The display name of this Workflow. This is what you'll see when you go to select it.
    pub(crate) name: String,
    /// A list of [`Step`]s to execute in order, stopping if any step fails.
    pub(crate) steps: Vec<Step>,
}

impl Workflow {
    /// Set `prerelease_label` for any steps that are `PrepareRelease` steps.
    pub(crate) fn set_prerelease_label(&mut self, prerelease_label: &str) {
        for step in &mut self.steps {
            step.set_prerelease_label(prerelease_label);
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum Verbose {
    Yes,
    No,
}

impl From<bool> for Verbose {
    fn from(verbose: bool) -> Self {
        if verbose {
            Verbose::Yes
        } else {
            Verbose::No
        }
    }
}

/// A collection of errors from running with the `--validate` option.
#[derive(Debug, Error, Diagnostic)]
#[error("There are problems with the defined workflows")]
pub struct ValidationErrorCollection {
    #[related]
    errors: Vec<Error>,
}

/// An error from running or validating a single workflow.
#[derive(Debug, thiserror::Error, Diagnostic)]
#[error("Problem with workflow {name}")]
pub struct Error {
    name: String,
    #[related]
    inner: [StepError; 1],
}

/// Run a series of [`Step`], each of which updates `state`.
pub(crate) fn run(workflow: Workflow, mut state: RunType, verbose: Verbose) -> Result<(), Error> {
    for step in workflow.steps {
        state = match step.run(state, verbose) {
            Ok(state) => state,
            Err(err) => {
                return Err(Error {
                    name: workflow.name,
                    inner: [err],
                });
            }
        };
    }
    Ok(())
}

#[allow(clippy::needless_pass_by_value)] // Lifetime errors if State is passed by ref.
pub(crate) fn validate(
    workflows: Vec<Workflow>,
    state: State,
    verbose: Verbose,
) -> Result<(), ValidationErrorCollection> {
    let errors = workflows
        .into_iter()
        .filter_map(|workflow| {
            run(
                workflow,
                RunType::DryRun {
                    state: state.clone(),
                    stdout: Box::new(sink()),
                },
                verbose,
            )
            .err()
        })
        .collect_vec();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationErrorCollection { errors })
    }
}

impl std::fmt::Display for Workflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.name)
    }
}
