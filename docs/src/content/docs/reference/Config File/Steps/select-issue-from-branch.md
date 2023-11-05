---
title: SelectIssueFromBranch
---

Attempt to parse issue info from the current branch for use in other steps (e.g., [`Command`]).

## Errors

This step will fail if the current git branch cannot be determined or the name of that branch does not match the expected format. This is only intended to be used on branches which were created using the [SwitchBranches] step.

## Example

```toml
[[workflows]]
name = "Finish some work"
    [[workflows.steps]]
    type = "SelectIssueFromBranch"

    [[workflows.steps]]
    type = "TransitionJiraIssue"
    status = "QA"
```

[`command`]: ./command
[switchbranches]: ./switch-branches
