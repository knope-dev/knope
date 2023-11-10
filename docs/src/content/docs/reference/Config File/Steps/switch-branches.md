---
title: SwitchBranches
---

Uses the name of the currently selected issue to checkout an existing or create a new branch for development.
If an existing branch isn't found,
Knope will prompt the user to select an existing local branch to base the new branch off of.
Remote branches aren't shown.

## Errors

This step fails if any of the following are true.

1. An issue wasn't yet selected in this workflow using [`SelectJiraIssue`] or [`SelectGitHubIssue`].
2. Current directory isn't a Git repository
3. There are uncommitted changes on the current branch. You must manually stash or commit any changes before performing this step.

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
