use std::collections::HashMap;

use execute::shell;
use serde::{Deserialize, Serialize};

use crate::{git::branch_name_from_issue, state, step::StepError, RunType, State};

/// Describes a value that you can replace an arbitrary string with when running a command.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum Variable {
    /// Uses the first supported version found in your project.
    Version,
    /// The generated branch name for the selected issue. Note that this means the workflow must
    /// already be in [`State::IssueSelected`] when this variable is used.
    IssueBranch,
}

/// Run the command string `command` in the current shell after replacing the keys of `variables`
/// with the values that the [`Variable`]s represent.
pub(crate) fn run_command(
    mut run_type: RunType,
    mut command: String,
    variables: Option<HashMap<String, Variable>>,
) -> Result<RunType, StepError> {
    let (state, dry_run_stdout) = match &mut run_type {
        RunType::DryRun { state, stdout } => (state, Some(stdout)),
        RunType::Real(state) => (state, None),
    };
    if let Some(variables) = variables {
        command = replace_variables(command, variables, state)?;
    }
    if let Some(stdout) = dry_run_stdout {
        writeln!(stdout, "Would run {command}")?;
        return Ok(run_type);
    }
    let status = shell(command).status()?;
    if status.success() {
        return Ok(run_type);
    }
    Err(StepError::CommandError(status))
}

/// Replace declared variables in the command string and return command.
fn replace_variables(
    mut command: String,
    variables: HashMap<String, Variable>,
    state: &State,
) -> Result<String, StepError> {
    for (var_name, var_type) in variables {
        match var_type {
            Variable::Version => {
                let package = if state.packages.len() > 1 {
                    return Err(StepError::TooManyPackages);
                } else if let Some(package) = state.packages.first() {
                    package
                } else {
                    return Err(StepError::no_defined_packages_with_help());
                };
                let version = if let Some(release) = package.prepared_release.as_ref() {
                    release.new_version.to_string()
                } else {
                    package
                        .get_version()?
                        .into_latest()
                        .ok_or(StepError::NoCurrentVersion)?
                        .to_string()
                };
                command = command.replace(&var_name, &version);
            }
            Variable::IssueBranch => match &state.issue {
                state::Issue::Initial => return Err(StepError::NoIssueSelected),
                state::Issue::Selected(issue) => {
                    command = command.replace(&var_name, &branch_name_from_issue(issue));
                }
            },
        }
    }
    Ok(command)
}

#[cfg(test)]
mod test_run_command {
    use tempfile::NamedTempFile;

    use super::*;
    use crate::State;

    #[test]
    fn test() {
        let file = NamedTempFile::new().unwrap();
        let command = format!("cat {}", file.path().to_str().unwrap());
        let result = run_command(
            RunType::Real(State::new(None, None, Vec::new())),
            command.clone(),
            None,
        );

        assert!(result.is_ok());

        file.close().unwrap();

        let result = run_command(
            RunType::Real(State::new(None, None, Vec::new())),
            command,
            None,
        );
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod test_replace_variables {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        issues::Issue,
        releases::{semver::Version, Package, Release},
        state,
    };

    fn packages() -> Vec<Package> {
        vec![Package {
            versioned_files: vec![PathBuf::from("Cargo.toml").try_into().unwrap()],
            changelog: Some(PathBuf::from("CHANGELOG.md").try_into().unwrap()),
            ..Package::default()
        }]
    }

    #[test]
    fn multiple_variables() {
        let command = "blah $$ branch_name".to_string();
        let mut variables = HashMap::new();
        variables.insert("$$".to_string(), Variable::Version);
        variables.insert("branch_name".to_string(), Variable::IssueBranch);
        let issue = Issue {
            key: "13".to_string(),
            summary: "1234".to_string(),
        };
        let expected_branch_name = branch_name_from_issue(&issue);
        let state = State {
            jira_config: None,
            github: state::GitHub::New,
            github_config: None,
            issue: state::Issue::Selected(issue),
            packages: packages(),
        };

        let command = replace_variables(command, variables, &state).unwrap();

        assert_eq!(
            command,
            format!(
                "blah {} {}",
                &state.packages[0]
                    .get_version()
                    .unwrap()
                    .into_latest()
                    .unwrap(),
                expected_branch_name
            )
        );
    }

    #[test]
    fn replace_version() {
        let command = "blah $$ other blah".to_string();
        let mut variables = HashMap::new();
        variables.insert("$$".to_string(), Variable::Version);
        let state = State::new(None, None, packages());

        let command = replace_variables(command, variables, &state).unwrap();

        assert_eq!(
            command,
            format!(
                "blah {} other blah",
                &state.packages[0]
                    .get_version()
                    .unwrap()
                    .into_latest()
                    .unwrap(),
            )
        );
    }

    #[test]
    fn replace_prepared_version() {
        let command = "blah $$ other blah".to_string();
        let mut variables = HashMap::new();
        variables.insert("$$".to_string(), Variable::Version);
        let mut state = State::new(None, None, packages());
        let version = Version::new(1, 2, 3, None);
        state.packages[0].prepared_release = Some(Release {
            new_version: version.clone(),
            new_changelog: String::new(),
        });

        let command = replace_variables(command, variables, &state).unwrap();

        assert_eq!(command, format!("blah {version} other blah"));
    }

    #[test]
    fn replace_issue_branch() {
        let command = "blah $$ other blah".to_string();
        let mut variables = HashMap::new();
        variables.insert("$$".to_string(), Variable::IssueBranch);
        let issue = Issue {
            key: "13".to_string(),
            summary: "1234".to_string(),
        };
        let expected_branch_name = branch_name_from_issue(&issue);
        let state = State {
            jira_config: None,
            github: state::GitHub::New,
            github_config: None,
            issue: state::Issue::Selected(issue),
            packages: Vec::new(),
        };

        let command = replace_variables(command, variables, &state).unwrap();

        assert_eq!(command, format!("blah {expected_branch_name} other blah"));
    }
}
