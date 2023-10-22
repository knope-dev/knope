![A purple binder, stuffed to the brim with papers. The word "Knope" is written on the front](favicon.png)

# Introduction

Knope is a CLI/CI tool which automates common tasks for developers. Things like creating changelogs, choosing and setting new versions, creating GitHub releases / tags, transitioning issues, creating and merging branches, creating pull requests... anything that is a repetitive, time-consuming task in your development cycle, this tool is made to speed up.

## How it Works

```admonish info
For some use-cases, you don't need to create a `knope.toml` file! If no file is detected, Knope will use the same config at runtime that it would create with `knope --generate`. Run `knope --generate` to see what you get for free, or check out the [default workflows](default_workflows.md).
```

You create a file called `knope.toml` in your project directory which defines some workflows. The format of this file is described in [the chapter on config][config], the key piece to which is the `workflows` array. You can get started quickly with `knope --generate` which will give you some starter workflows.

In order to run a workflow (whether via a custom file or a [default workflow](default_workflows.md)), you run `knope <workflow name>`. For example, `knope release` will run a workflow named `release` (and error if such a workflow does not exist). You can also run `knope --help` to see a list of available workflows.

## CLI Arguments

Except for a few options, `knope` must always be run with one positional argument, the name of the workflow to be run. So `knope release` expects there to be a workflow named `release` (as there is in the [default workflows](default_workflows.md)). Here are all the options that can be passed, note that some of them are situational (e.g., only available when running a relevant workflow):

### `--help`

Prints out a help message containing available workflows and options, then exits. This can be run without any positional workflow argument.

### `--version`

Prints out the version of `knope` and exits. This can be run without any positional workflow argument.

### `--verbose`

Generally makes `knope` spit out a _lot_ of extra detail to stdout to help with diagnosing issues.

### `--generate`

Generates a `knope.toml` file in the current directory. _This cannot be used if there is already a `knope.toml` file present._

### `--upgrade`

Upgrades your `knope.toml` file from deprecated syntax to the new syntax in preparation for the next breaking release. _This can only be used if you have a `knope.toml` file, not if you are using the [default workflows](default_workflows.md)._

### `--validate`

Checks your `knope.toml` to make sure every workflow in it is valid, then exits. This could be useful to run in CI to make sure that your config is always valid. The exit code of this command will be 0 only if the config is valid. _This cannot be used if there is no `knope.toml` file present._

### `--dry-run`

Pretends to run the selected workflow (one must be provided), but will not actually perform any work (for example, no external commands, file I/O, or API calls). Detects the same errors as `--validate` but also outputs info about what _would_ happen to the standard output (likely your terminal window). For example, to see what `knope release` _would_ do without creating an actual release, run `knope release --dry-run`.

#### `--prerelease-label`

Overrides the `prerelease_label` for any [`PrepareRelease`] step run. _This option can only be provided after a workflow which contains a [`PrepareRelease`] step._

#### `--override-version`

Allows you to manually determine the next version for a [`BumpVersion`] or [`PrepareRelease`] instead of using a semantic versioning rule. This option can only be provided after a workflow which contains a relevant step. This has two formats, depending on whether there is [one package](config/packages.md#a-single-package-with-a-single-versioned-file) or [multiple packages](config/packages.md#multiple-packages).

If the single-package format is used (as it is for the [default workflows](default_workflows.md), `--override-version 1.0.0` will set the version to `1.0.0`.

If the multi-package syntax is used (**even if only one package is configured with it**), you must specify the name of each package that should be overriden. For example, `--override-version first-package=1.0.0 --override-version second-package=2.0.0` will set the version of `first-package` to `1.0.0` and `second-package` to `2.0.0`, erroring if either of those packages is not configured.

### Environment Variables

These are all the environment variables that Knope will look for when running workflows.

1. `KNOPE_PRERELEASE_LABEL` works just like the `--prerelease-label` option. Note that the option takes precedence over the environment variable.
2. `GITHUB_TOKEN` will be used to load credentials from GitHub for [GitHub config].

## Features

More detail on everything this program can do can be found by digging into [config] but here's a rough (incomplete) summary:

1. Select issues from Jira or GitHub to work on, transition and create branches from them.
2. Do some basic git commands like switching branches or rebasing.
3. Bump the version of your project using semantic rules.
4. Bump your version AND generate a Changelog entry from conventional commits.
5. Do whatever you want by running arbitrary shell commands and substituting data from the project!

## Concepts

You define a [config] file named `knope.toml` which has some metadata (e.g. [package definitions]) about your project, as well as a set of [workflows][workflow]. Each [workflow] consists of a series of [steps][step] that will execute in order, stopping if any step fails. Some [steps][step] require other steps to be run before they are.

## The Name

Knope (pronounced like "nope") is a reference to the character Leslie Knope from the TV show Parks and Recreation. She loves doing the hard, tedious work that most people don't like doing, and she's very good at it, just like this tool!

## The Logo

The logo is a binder in reference to Leslie Knope's love of binders. The binder is also analogous to the `knope.toml` file which defines all the workflows for your project.

[config]: config/config.md
[package definitions]: config/packages.md
[workflow]: config/workflow.md
[step]: config/step/step.md
[`preparerelease`]: config/step/PrepareRelease.md
[`bumpversion`]: config/step/BumpVersion.md
[github config]: config/github.md
