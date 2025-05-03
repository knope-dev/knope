---
title: Variables
---

You can customize some steps with variables—pieces of context that Knope can substitute into strings.
You configure variables with both the string to replace and the name of the variable that
will replace it. For example, if you wanted to insert the current package
version into a command, you might use a `{"version": "Version"}` variable config. This would replace any instance
of the string `version` with `Version`.

## Defaults

Any part of `knope.toml` which _can_ accept variables will replace the following variables _by default_.
Anywhere the string on the left appears in the content, it'll be replaced.

- `$version`: [`Version`](#version)
- `$changelog`: [`ChangelogEntry`](#changelogentry)

These defaults are only in place if custom variables aren't defined. To disable to defaults, define variables, even
as an empty map:

```toml
[[workflows.steps]]
type = "Command"
command = "echo $version"
variables = {} # Disable default `$version` variable substitution
```

## `Version`

`Version` will try to parse the current package version.

:::caution
You can only use this variable with the single `[package]` config, not with `[packages.<name>]`.
:::

## `ChangelogEntry`

`ChangelogEntry` is the content of the changelog (if any) for the version in the [`Version`](#version) variable.

:::caution
You can only use this variable with the single `[package]` config, not with `[packages.<name>]`.
:::

## `IssueBranch`

`IssueBranch` will produce the same branch name that the [`SwitchBranches`] step would produce. You must have already
selected an issue in this workflow using [`SelectJiraIssue`], [`SelectGitHubIssue`], or [`SelectIssueFromBranch`] before
using this variable.

[`SwitchBranches`]: /reference/config-file/steps/switch-branches
[`SelectJiraIssue`]: /reference/config-file/steps/select-jira-issue
[`SelectGitHubIssue`]: /reference/config-file/steps/select-github-issue
[`SelectIssueFromBranch`]: /reference/config-file/steps/select-issue-from-branch
