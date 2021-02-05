# SelectJiraIssue

Search for Jira issues by status and display the list of them in the terminal. User is allowed to select one issue which will then change the workflow's [state] to [IssueSelected].

## Errors

This step will fail if any of the following are true:

1. The [state] is already in [IssueSelected] before it executes.
2. Dobby cannot communicate with the [configured Jira URL][jira].
3. User does not select an issue (e.g. by pressing `Esc`).
4. There is no [Jira config][jira] set.

## Example

```toml
[[workflows]]
name = "Start some work"
    [[workflows.steps]]
    type = "SelectJiraIssue"
    status = "Backlog"
```

[state]: ../../state/state.md
[issueselected]: ../../state/IssueSelected.md
[jira]: ../jira.md
