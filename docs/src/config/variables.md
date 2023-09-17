# Variables

Some steps, notably [`Command`] and [`CreatePullRequest`] allow you to use variables in their configuration. Typically, this allows for string substitution with some context that Knope has. Variables are always configured by providing both the string that should be replaced and the name of the variable that should replace it, so you can customize your own syntax. For example, if you wanted to insert the current package version into a command, you might provide a `{"version": "Version"}` variable config. This would replace any instance of the string `version` with `Version`. If you wanted a bash-like syntax, you might use `{"$version": "Version"}` instead—pick whatever works best for you.

## `Version`

`Version` will attempt to parse the current package version and substitute that string. For example, you might use this to get the new version after running a [`PrepareRelease`] step.

```admonish warning
This variable can only be used when a single `[package]` is configured, there is currently no equivalent for multi-package projects.
```

## `ChangelogEntry`

`ChangelogEntry` is the content of the changelog (if any) for the version that is indicated by the [`Version`](#version) variable. This follows the same rules as the [`Release`] step for creating a [GitHub changelog](./step/Release.md#github-release-notes), with the exception that it cannot use GitHub's auto-generated release notes. If no changelog entry can be found, the step fails.

```admonish warning
This variable can only be used when a single `[package]` is configured, there is currently no equivalent for multi-package projects.
```

## `IssueBranch`

`IssueBranch` will provide the same branch name that the [`SwitchBranches`] step would produce. You must have already selected an issue in this workflow using [`SelectJiraIssue`], [`SelectGitHubIssue`], or [`SelectIssueFromBranch`] before using this variable.

[`Command`]: ./step/Command.md
[`CreatePullRequest`]: ./step/CreatePullRequest.md
[`PrepareRelease`]: ./step/PrepareRelease.md
[`Release`]: ./step/Release.md
[`SwitchBranches`]: ./step/SwitchBranches.md
[`SelectJiraIssue`]: ./step/SelectJiraIssue.md
[`SelectGitHubIssue`]: ./step/SelectGitHubIssue.md
[`SelectIssueFromBranch`]: ./step/SelectIssueFromBranch.md
