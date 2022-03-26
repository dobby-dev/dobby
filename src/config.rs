use std::fs;

use miette::{IntoDiagnostic, Result, WrapErr};
use serde::{Deserialize, Serialize};
use velcro::{hash_map, vec};

use crate::step::{PrepareRelease, Step};
use crate::workflow::Workflow;
use crate::{command, git};

#[derive(Deserialize, Debug, Serialize)]
pub(crate) struct Config {
    /// The list of defined workflows that are selectable
    pub(crate) workflows: Vec<Workflow>,
    /// Optional configuration for Jira
    pub(crate) jira: Option<Jira>,
    /// Optional configuration to talk to GitHub
    pub(crate) github: Option<GitHub>,
}

impl Config {
    const CONFIG_PATH: &'static str = "dobby.toml";

    /// Create a Config from a TOML file.
    ///
    /// ## Errors
    /// 1. Provided path is not found
    /// 2. Cannot parse file contents into a Config
    pub(crate) fn load() -> Result<Self> {
        let contents = fs::read_to_string(Self::CONFIG_PATH)
            .into_diagnostic()
            .wrap_err_with(|| {
                format!(
                    "Could not find {CONFIG_PATH}",
                    CONFIG_PATH = Self::CONFIG_PATH
                )
            })?;
        toml::from_str(&contents)
            .into_diagnostic()
            .wrap_err("Invalid TOML when parsing config")
    }
}

/// Generate a brand new config file for the project in the current directory.
pub(crate) fn generate() -> Result<()> {
    let variables = hash_map! {
        String::from("$version"): command::Variable::Version,
    };

    let release_steps = match git::get_first_remote() {
        Some(remote) if remote.contains("github.com") => {
            vec![
                Step::Command {
                    command: String::from("git add . && git commit -m \"chore: prepare release $version\" && git push"),
                    variables: Some(variables),
                },
                Step::Release,
            ]
        },
        _ => vec![
            Step::Command {
                command: String::from("git add . && git commit -m \"chore: prepare release $version\" && git push && git tag -m $version && git push --tags"),
                variables: Some(variables),
            }
        ],
    };

    let contents = toml::to_string(&Config {
        workflows: vec![Workflow {
            name: String::from("release"),
            steps: vec![
                Step::PrepareRelease(PrepareRelease {
                    changelog_path: String::from("CHANGELOG.md"),
                    prerelease_label: None,
                }),
                ..release_steps,
            ],
        }],
        jira: None,
        github: None,
    })
    .unwrap();
    fs::write(Config::CONFIG_PATH, contents).into_diagnostic()
}

/// Config required for steps that interact with Jira.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct Jira {
    /// The URL to your Atlassian instance running Jira
    pub(crate) url: String,
    /// The key of the Jira project to filter on (the label of all issues)
    pub(crate) project: String,
}

/// Details needed to use steps that interact with GitHub.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct GitHub {
    /// The user or organization that owns the `repo`.
    pub(crate) owner: String,
    /// The name of the repository in GitHub that this project is utilizing
    pub(crate) repo: String,
}
