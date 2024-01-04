---
title: SelectGiteaIssue
---

Search for Gitea issues by status and display the list of them in the terminal.
Selecting an issue enables other steps to use the issue's information (for example, [`SwitchBranches`]).

## Errors

This step will fail if any of the following are true:

1. Knope can't communicate with the Gitea instance.
2. There is no [Gitea config] set.
3. User doesn't select an issue.

## Example

```toml
[[workflows]]
name = "Start some work"
    [[workflows.steps]]
    type = "SelectGiteaIssue"
    label = "selected"
```

[Gitea config]: /reference/config-file/gitea
[`switchbranches`]: /reference/config-file/steps/switch-branches
