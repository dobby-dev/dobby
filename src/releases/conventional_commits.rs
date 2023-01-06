use std::io::Write;

use git_conventional::{Commit, Type};
use log::debug;

use crate::git::{add_files, get_commit_messages_after_last_stable_version};
use crate::releases::semver::{Label, PackageVersion};
use crate::releases::Package;
use crate::step::StepError;
use crate::{state, step, RunType};

use super::changelog::{add_version_to_changelog, new_changelog_lines};
use super::semver::{bump_version, ConventionalRule, Rule};
use super::Release;

#[derive(Debug)]
struct ConventionalCommits {
    rule: Option<ConventionalRule>,
    features: Vec<String>,
    fixes: Vec<String>,
    breaking_changes: Vec<String>,
}

impl ConventionalCommits {
    fn from_commit_messages(
        commit_messages: &[String],
        consider_scopes: bool,
        package: &Package,
    ) -> Self {
        let commits = commit_messages
            .iter()
            .filter_map(|message| Commit::parse(message.trim()).ok())
            .filter(|commit| {
                if !consider_scopes {
                    return true;
                }
                match (commit.scope(), &package.scopes) {
                    (None, _) => true,
                    (Some(_), None) => false,
                    (Some(scope), Some(scopes)) => scopes.contains(&scope.to_string()),
                }
            })
            .collect();
        debug!("Selected commits: {:?}", commits);
        Self::from_commits(commits)
    }

    fn from_commits(commits: Vec<Commit>) -> Self {
        let mut rule = None;
        let mut features = Vec::new();
        let mut fixes = Vec::new();
        let mut breaking_changes = Vec::new();

        for commit in commits {
            if let Some(breaking_message) = commit.breaking_description() {
                if !matches!(rule, Some(ConventionalRule::Major)) {
                    debug!(
                        "breaking change \"{}\" results in Major rule selection",
                        breaking_message
                    );
                    rule = Some(ConventionalRule::Major);
                }
                breaking_changes.push(breaking_message.to_string());
                if breaking_message == commit.description() {
                    // There is no separate breaking change message, so the normal description is used.
                    // Don't include the same message elsewhere.
                    continue;
                }
            }

            if commit.type_() == Type::FEAT {
                features.push(commit.description().to_string());
                if !matches!(rule, Some(ConventionalRule::Major)) {
                    debug!(
                        "commit \"{}\" results in Minor rule selection",
                        commit.description()
                    );
                    rule = Some(ConventionalRule::Minor);
                }
            } else if commit.type_() == Type::FIX {
                if rule.is_none() {
                    debug!(
                        "commit \"{}\" results in Patch rule selection",
                        commit.description()
                    );
                    rule = Some(ConventionalRule::Patch);
                }
                fixes.push(commit.description().to_string());
            }
        }

        ConventionalCommits {
            rule,
            features,
            fixes,
            breaking_changes,
        }
    }
}

#[cfg(test)]
mod test_conventional_commits {
    use super::*;

    #[test]
    fn non_breaking_features() {
        let commits = vec![
            Commit::parse("feat: add a feature").unwrap(),
            Commit::parse("feat: another feature").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, Some(ConventionalRule::Minor));
        assert_eq!(
            conventional_commits.features,
            vec![
                String::from("add a feature"),
                String::from("another feature")
            ]
        );
        assert_eq!(conventional_commits.fixes, Vec::<String>::new());
        assert_eq!(conventional_commits.breaking_changes, Vec::<String>::new());
    }

    #[test]
    fn non_breaking_fixes() {
        let commits = vec![
            Commit::parse("fix: a bug").unwrap(),
            Commit::parse("fix: another bug").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, Some(ConventionalRule::Patch));
        assert_eq!(
            conventional_commits.fixes,
            vec![String::from("a bug"), String::from("another bug")]
        );
        assert_eq!(conventional_commits.features, Vec::<String>::new());
        assert_eq!(conventional_commits.breaking_changes, Vec::<String>::new());
    }

    #[test]
    fn mixed_fixes_and_features() {
        let commits = vec![
            Commit::parse("fix: a bug").unwrap(),
            Commit::parse("feat: add a feature").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, Some(ConventionalRule::Minor));
        assert_eq!(conventional_commits.fixes, vec![String::from("a bug")]);
        assert_eq!(
            conventional_commits.features,
            vec![String::from("add a feature")]
        );
        assert_eq!(conventional_commits.breaking_changes, Vec::<String>::new());
    }

    #[test]
    fn breaking_feature() {
        let commits = vec![
            Commit::parse("fix: a bug").unwrap(),
            Commit::parse("feat!: add a feature").unwrap(),
            Commit::parse("feat: add another feature").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, Some(ConventionalRule::Major));
        assert_eq!(conventional_commits.fixes, vec![String::from("a bug")]);
        assert_eq!(
            conventional_commits.features,
            vec![String::from("add another feature")]
        );
        assert_eq!(
            conventional_commits.breaking_changes,
            vec![String::from("add a feature")]
        );
    }

    #[test]
    fn breaking_fix() {
        let commits = vec![
            Commit::parse("fix!: a bug").unwrap(),
            Commit::parse("fix: another bug").unwrap(),
            Commit::parse("feat: add a feature").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, Some(ConventionalRule::Major));
        assert_eq!(
            conventional_commits.fixes,
            vec![String::from("another bug")]
        );
        assert_eq!(
            conventional_commits.features,
            vec![String::from("add a feature")]
        );
        assert_eq!(
            conventional_commits.breaking_changes,
            vec![String::from("a bug")]
        );
    }

    #[test]
    fn fix_with_separate_breaking_message() {
        let commits = vec![
            Commit::parse("fix: a bug\n\nBREAKING CHANGE: something broke").unwrap(),
            Commit::parse("fix: another bug").unwrap(),
            Commit::parse("feat: add a feature").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, Some(ConventionalRule::Major));
        assert_eq!(
            conventional_commits.fixes,
            vec![String::from("a bug"), String::from("another bug")]
        );
        assert_eq!(
            conventional_commits.features,
            vec![String::from("add a feature")]
        );
        assert_eq!(
            conventional_commits.breaking_changes,
            vec![String::from("something broke")]
        );
    }

    #[test]
    fn feature_with_separate_breaking_message() {
        let commits = vec![
            Commit::parse("feat: add a feature\n\nBREAKING CHANGE: something broke").unwrap(),
            Commit::parse("fix: a bug").unwrap(),
            Commit::parse("feat: add another feature").unwrap(),
        ];
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, Some(ConventionalRule::Major));
        assert_eq!(conventional_commits.fixes, vec![String::from("a bug")]);
        assert_eq!(
            conventional_commits.features,
            vec![
                String::from("add a feature"),
                String::from("add another feature")
            ]
        );
        assert_eq!(
            conventional_commits.breaking_changes,
            vec![String::from("something broke")]
        );
    }

    #[test]
    fn no_commits() {
        let commits = Vec::<Commit>::new();
        let conventional_commits = ConventionalCommits::from_commits(commits);
        assert_eq!(conventional_commits.rule, None);
        assert_eq!(conventional_commits.fixes, Vec::<String>::new());
        assert_eq!(conventional_commits.features, Vec::<String>::new());
        assert_eq!(conventional_commits.breaking_changes, Vec::<String>::new());
    }

    #[test]
    fn dont_consider_scopes() {
        let commits = [
            "feat(wrong_scope)!: Wrong scope breaking change!",
            "fix: No scope",
        ]
        .map(String::from);
        let conventional_commits = ConventionalCommits::from_commit_messages(
            &commits,
            false,
            &Package {
                versioned_files: vec![],
                changelog: None,
                name: None,
                scopes: Some(vec![String::from("scope")]),
            },
        );
        assert_eq!(conventional_commits.rule, Some(ConventionalRule::Major));
    }

    #[test]
    fn consider_scopes_but_none_defined() {
        let commits = [
            "feat(scope)!: Wrong scope breaking change!",
            "fix: No scope",
        ]
        .map(String::from);
        let conventional_commits = ConventionalCommits::from_commit_messages(
            &commits,
            true,
            &Package {
                versioned_files: vec![],
                changelog: None,
                name: None,
                scopes: None,
            },
        );
        assert_eq!(conventional_commits.rule, Some(ConventionalRule::Patch));
    }

    #[test]
    fn consider_scopes() {
        let commits = [
            "feat(wrong_scope)!: Wrong scope breaking change!",
            "feat(scope): Right scope feature",
            "fix: No scope",
        ]
        .map(String::from);
        let conventional_commits = ConventionalCommits::from_commit_messages(
            &commits,
            true,
            &Package {
                versioned_files: vec![],
                changelog: None,
                name: None,
                scopes: Some(vec![String::from("scope")]),
            },
        );
        assert_eq!(conventional_commits.rule, Some(ConventionalRule::Minor));
    }
}

fn get_conventional_commits_after_last_stable_version(
    package: &Package,
    consider_scopes: bool,
) -> Result<ConventionalCommits, StepError> {
    let commit_messages = get_commit_messages_after_last_stable_version(&package.name)?;
    Ok(ConventionalCommits::from_commit_messages(
        &commit_messages,
        consider_scopes,
        package,
    ))
}

pub(crate) fn update_project_from_conventional_commits(
    run_type: RunType,
    prepare_release: &step::PrepareRelease,
) -> Result<RunType, StepError> {
    let (mut state, mut dry_run_stdout) = match run_type {
        RunType::DryRun { state, stdout } => (state, Some(stdout)),
        RunType::Real(state) => (state, None),
    };
    if state.packages.is_empty() {
        return Err(StepError::no_defined_packages_with_help());
    }
    let consider_scopes = state
        .packages
        .iter()
        .any(|package| package.scopes.is_some());
    for package in &mut state.packages {
        let release = prepare_release_for_package(
            package.clone(),
            consider_scopes,
            prepare_release.prerelease_label.as_ref(),
            dry_run_stdout.as_mut(),
        )?;
        if let Some(release) = release {
            state.releases.push(state::Release::Prepared(release));
        }
    }
    if let Some(dry_run_stdout) = dry_run_stdout {
        Ok(RunType::DryRun {
            state,
            stdout: dry_run_stdout,
        })
    } else if state.releases.is_empty() {
        Err(StepError::NoRelease)
    } else {
        Ok(RunType::Real(state))
    }
}

fn prepare_release_for_package(
    package: Package,
    consider_scopes: bool,
    prerelease_label: Option<&Label>,
    dry_run_stdout: Option<&mut Box<dyn Write>>,
) -> Result<Option<Release>, StepError> {
    let ConventionalCommits {
        rule,
        features,
        fixes,
        breaking_changes,
    } = get_conventional_commits_after_last_stable_version(&package, consider_scopes)?;
    let rule = if let Some(rule) = rule {
        rule
    } else {
        return Ok(None);
    };

    let rule = if let Some(label) = prerelease_label {
        Rule::Pre {
            label: label.clone(),
            stable_rule: rule,
        }
    } else {
        Rule::from(rule)
    };
    let PackageVersion { package, version } =
        bump_version(&rule, dry_run_stdout.is_some(), package)?;
    let new_version_string = version.to_string();
    let new_changes =
        new_changelog_lines(&new_version_string, &fixes, &features, &breaking_changes);

    let release = Release {
        version,
        changelog: new_changes.join("\n"),
        package_name: package.name,
    };
    let changelog = package.changelog.as_ref();

    if let Some(stdout) = dry_run_stdout {
        writeln!(
            stdout,
            "Would bump {} version to {}",
            release.package_name.as_deref().unwrap_or("package"),
            new_version_string
        )?;
        if let Some(changelog) = changelog {
            writeln!(
                stdout,
                "Would add the following to {}: \n{}",
                changelog.path.display(),
                new_changes.join("\n")
            )?;
        }
        Ok(Some(release))
    } else {
        if let Some(changelog) = changelog {
            let contents = add_version_to_changelog(&changelog.content, &new_changes);
            std::fs::write(&changelog.path, contents)?;
            add_files(&[&changelog.path])?;
        }
        Ok(Some(release))
    }
}
