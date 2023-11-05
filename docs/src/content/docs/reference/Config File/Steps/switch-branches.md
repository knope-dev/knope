---
title: SwitchBranches
---

Uses the name of the currently selected issue to checkout an existing or create a new branch for development. If an existing branch is not found, the user will be prompted to select an existing local branch to base the new branch off of. Remote branches are not shown.

## Errors

This step fails if any of the following are true.

1. An issue was not previously selected in this workflow using [`SelectJiraIssue`] or [`SelectGitHubIssue`].
1. Current directory is not a Git repository
1. There is uncommitted work on the current branch. You must manually stash or commit any changes before performing this step.

## Example

```toml
[[workflows]]
name = "Start some work"
    [[workflows.steps]]
    type = "SelectJiraIssue"
    status = "Backlog"

    [[workflows.steps]]
    type = "SwitchBranches"
```

[`selectjiraissue`]: /reference/config-file/steps/select-jira-issue
[`selectgithubissue`]: /reference/config-file/steps/select-github-issue
