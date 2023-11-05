---
title: SelectJiraIssue
---

Search for Jira issues by status and display the list of them in the terminal. User is allowed to select one issue which can then be used in future steps in this workflow (e.g., [`Command`] or [`SwitchBranches`]).

## Errors

This step will fail if any of the following are true:

1. knope cannot communicate with the [configured Jira URL][jira].
2. User does not select an issue (e.g. by pressing `Esc`).
3. There is no [Jira config][jira] set.

## Example

```toml
[[workflows]]
name = "Start some work"
    [[workflows.steps]]
    type = "SelectJiraIssue"
    status = "Backlog"
```

[`command`]: ./command
[`switchbranches`]: ./switch-branches
[jira]: ../jira.md
