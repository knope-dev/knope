# Command step

Run a command in your current shell after optionally replacing some variables. This step is here to cover the infinite things you might want to do that knope does not yet know how to do itself. If you have a lot of these steps or a complex `command`, we recommend you write a script in something like Bash or Python, then simply call that script with a command.

## Example

If the current version for your project is "1.0.0", the following workflow step will run `git tag v.1.0.0` in your current shell.

```toml
[[workflows.steps]]
type = "Command"
command = "git tag v.version"
variables = {"version" = "Version"}
```

## Variables

The `variables` attribute of this step is an object where the key is the string you wish to substitute and the value is one of the available variables listed below. **take care when selecting a key to replace** as _any_ matching string that is found will be replaced. The order of this replacement is not guaranteed, so it is also possible for multiple variables to conflict with one another.

### Available Variables

1. `Version` will attempt to parse the project version using the same method as the [BumpVersion] step and substitute that string. It will select the first version found in any of the supported file names / formats to use for substitution. If no version can be found and parsed, this step will fail.

1. `IssueBranch` will provide the same branch name that the [SwitchBranches] step would produce. You must have already selected an issue in this workflow using [`SelectJiraIssue`], [`SelectGitHubIssue`], or [`SelectIssueFromBranch`] before using this variable.

[bumpversion]: ./BumpVersion.md
[switchbranches]: ./SwitchBranches.md
[`selectjiraissue`]: ./SelectJiraIssue.md
[`selectgithubissue`]: ./SelectGitHubIssue.md
[`selectissuefrombranch`]: ./SelectIssueFromBranch.md
