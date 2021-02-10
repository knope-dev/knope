# SelectIssueFromBranch step

Attempt to parse issue info from the current branch name and change the workflow's state to [IssueSelected].

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

[issueselected]: ../../state/IssueSelected.md
[switchbranches]: ./SwitchBranches.md
