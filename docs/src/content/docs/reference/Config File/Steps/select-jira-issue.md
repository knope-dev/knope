---
title: SelectJiraIssue
---

Search for Jira issues by status and display the list of them in the terminal.
Knope prompts the user to select one issue, and then can use it in future steps in this workflow (for example, [`Command`] or [`SwitchBranches`]).

## Errors

This step will fail if any of the following are true:

1. Knope can't communicate with the [configured Jira URL][jira].
2. User doesn't select an issue (for example, by pressing `Esc`).
3. There is no [Jira config][jira] set.

## Example

```toml
[[workflows]]
name = "Start some work"
    [[workflows.steps]]
    type = "SelectJiraIssue"
    status = "Backlog"
```

[`command`]: /reference/config-file/steps/command
[`switchbranches`]: /reference/config-file/steps/switch-branches
[jira]: /reference/config-file/jira
