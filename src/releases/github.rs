use std::io::Write;

use serde::Serialize;

use crate::app_config::get_or_prompt_for_github_token;
use crate::config::GitHub;
use crate::releases::Release;
use crate::state::GitHub::{Initialized, New};
use crate::step::StepError;
use crate::{RunType, State};

pub(crate) fn release(
    mut state: State,
    dry_run_stdout: Option<Box<dyn Write>>,
    github_config: &GitHub,
    release: &Release,
) -> Result<RunType, StepError> {
    let version_string = release.version.to_string();

    let github_release = GitHubRelease {
        tag_name: &format!("v{version_string}"),
        name: &version_string,
        body: &release.changelog,
        prerelease: !release.version.pre.is_empty(),
    };

    if let Some(mut stdout) = dry_run_stdout {
        let release_type = if github_release.prerelease {
            "prerelease"
        } else {
            "release"
        };
        writeln!(
            stdout,
            "Would create a {} on GitHub with name and tag {} and body:\n{}",
            release_type, github_release.tag_name, github_release.body
        )?;
        return Ok(RunType::DryRun { stdout, state });
    }

    let token = match state.github {
        Initialized { token } => token,
        New => get_or_prompt_for_github_token()?,
    };

    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/releases",
        owner = github_config.owner,
        repo = github_config.repo,
    );
    let token_header = format!("token {}", &token);

    let response = ureq::post(&url)
        .set("Authorization", &token_header)
        .send_json(github_release)?;

    if response.status() != 201 {
        return Err(StepError::ApiResponseError(None));
    }
    state.github = Initialized { token };
    Ok(RunType::Real(state))
}

#[derive(Serialize)]
struct GitHubRelease<'a> {
    tag_name: &'a str,
    name: &'a str,
    body: &'a str,
    prerelease: bool,
}
