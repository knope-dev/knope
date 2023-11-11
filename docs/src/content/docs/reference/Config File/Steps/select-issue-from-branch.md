---
title: SelectIssueFromBranch
---

Try to parse issue info from the current branch for use in other steps (for example [`Command`]).

## Errors

This step will fail if Knope can't determine the current git branch,
or the name of that branch doesn't match the expected format.
This is only intended to be used on branches which Knope created with the [SwitchBranches] step.

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

[`command`]: /reference/config-file/steps/command
[switchbranches]: /reference/config-file/steps/switch-branches
