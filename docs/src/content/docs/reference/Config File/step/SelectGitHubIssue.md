---
title: SelectGitHubIssue
---

Search for GitHub issues by status and display the list of them in the terminal. Selecting an issue allows for other steps to use the issue's information (e.g., [`SwitchBranches`]).

## Errors

This step will fail if any of the following are true:

1. knope cannot communicate with GitHub.
2. There is no [GitHub config] set.
3. User does not select an issue.

## Example

```toml
[[workflows]]
name = "Start some work"
    [[workflows.steps]]
    type = "SelectGitHubIssue"
    label = "selected"
```

[github config]: ../github.md
[`switchbranches`]: ./SwitchBranches.md
