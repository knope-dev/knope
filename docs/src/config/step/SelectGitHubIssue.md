# SelectGitHubIssue Step

Search for GitHub issues by status and display the list of them in the terminal. User is allowed to select one issue which will then change the workflow's state to
[IssueSelected].

## Errors

This step will fail if any of the following are true:

1. The workflow is already in [IssueSelected] state before it executes.
2. knope cannot communicate with GitHub.
3. There is no [GitHub config] set.
4. User does not select an issue.

## Example

```toml
[[workflows]]
name = "Start some work"
    [[workflows.steps]]
    type = "SelectGitHubIssue"
    label = "selected"
```

[issueselected]: ../../state/IssueSelected.md
[github config]: ../github.md
