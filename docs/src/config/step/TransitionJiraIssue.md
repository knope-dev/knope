# TransitionJiraIssue Step

Transition a Jira issue to a new status.

## Errors

This step will fail when any of the following are true:

1. The workflow is not yet in [IssueSelected] state.
2. Cannot communicate with Jira.
3. The configured status is invalid for the issue.

## Example

```toml
[[workflows]]
name = "Start some work"
    [[workflows.steps]]
    type = "SelectJiraIssue"
    status = "Backlog"

    [[workflows.steps]]
    type = "TransitionJiraIssue"
    status = "In Progress"
```

[issueselected]: ../../state/IssueSelected.md
