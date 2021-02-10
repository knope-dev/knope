# Step

A step is the atomic unit of work that Dobby operates on. In a [workflow], steps will be executed sequentially until one fails or all steps are completed.

In it's simplest form, a step is declared in a [workflow] like this:

```toml
[[workflows]]
name = "My Workflow"
    [[workflows.steps]]
    type = "AStepType"
    more_info = "Something"
```

Where `type` matches one of the available steps listed below. Some steps also can take additional parameters in config, those go right underneath `type` like `more_info` above.

## Available Steps

- [SelectJiraIssue](./SelectJiraIssue.md)
- [TransitionJiraIssue](./TransitionJiraIssue.md)
- [SelectGitHubIssue](./SelectGitHubIssue.md)
- [SelectIssueFromBranch](./SelectIssueFromBranch.md)
- [SwitchBranches](./SwitchBranches.md)
- [RebaseBranch](./RebaseBranch.md)
- [BumpVersion](./BumpVersion.md)
- [Command](./Command.md)
- [UpdateProjectFromCommits](./UpdateProjectFromCommits.md)

[workflow]: ../workflow.md
