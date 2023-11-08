---
title: Command Line Arguments
---

## Workflow

Knope only accepts a single positional argument (one which doesn't begin with `-`),
and it must be the name of a defined workflow. `knope release` runs a workflow named release.

## Non-workflow arguments

These arguments cause Knope to do something _other_ than running a workflow.

### `--help`

Prints a message describing everything that Knope can do, including descriptions of other _available_ arguments.

If run _after_ a workflow name (for example, `knope release --help`), Knope prints information relevant to that workflow.

### `--version`

Prints the version of `knope` and exits.

### `--generate`

Creates a `knope.toml` file then exits. Not available if a `knope.toml` file already exists.

### `--upgrade`

Updates the `knope.toml` file from any deprecated (but still supported) syntax to the newer syntax.
This option is unavailable if no `knope.toml` file is present.

### `--validate`

Checks that the `knope.toml` file is valid. Unavailable if there is no `knope.toml` file in the current directory.

## Workflow modifiers

Arguments that change the behavior of a workflow, the workflow will still run.

### `--verbose`

Print out more info at every step, aiding in debugging.

### `--dry-run`

Don't change any files on disk, make any network calls, or call any external commands.
Instead, print out what _would_ happen without the `--dry-run` flag.

### `--prerelease-label`

Set or override a `prerelease_label` for any [`PrepareRelease`] step.
Only available for workflows that contain the [`PrepareRelease`] step (like the default `release` workflow)

You can also set this with the [`KNOPE_PRERELEASE_LABEL`](/reference/environment-variables#knope_prerelease_label) environment variable.
This option takes precedence over that.

### `--override-version`

Manually set a version for all [`BumpVersion`] and [`PrepareRelease`] steps instead of using semantic rules.
Only available for workflows that contain one of those steps, like the built in `release` workflow.

If the [single-package syntax] is used, provide a single semantic version, like `--override-version 1.0.0`.

If the [multi-package syntax] is used (even if only one package is configured with it),
you must specify the name of each package that should be overridden.
This option can also be provided more than once.
For example, `--override-version first-package=1.0.0 --override-version second-package=2.0.0`
will set the version of `first-package` to 1.0.0 and `second-package` to 2.0.0,
producing an error if either of those packages isn't configured.

[`BumpVersion`]: /reference/config-file/steps/bump-version
[`PrepareRelease`]: /reference/config-file/steps/prepare-release
