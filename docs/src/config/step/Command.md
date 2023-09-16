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

The `variables` attribute of this step is an object where the key is the string you wish to substitute and the value is one of the [available variables](../variables.md). **take care when selecting a key to replace** as _any_ matching string that is found will be replaced. Replacements occur in the order they are declared in the config, so earlier substitutions may be replaced by later ones.

[bumpversion]: ./BumpVersion.md
[switchbranches]: ./SwitchBranches.md
[`selectjiraissue`]: ./SelectJiraIssue.md
[`selectgithubissue`]: ./SelectGitHubIssue.md
[`selectissuefrombranch`]: ./SelectIssueFromBranch.md
