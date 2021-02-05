# IssueSelected State

This [state] indicates that work is being done against an issue in some project management platform. Currently the supported platforms are GitHub and Jira. Any [step] which requires this state must be proceeded by a step which triggers this state.

## Triggered By

- [SelectJiraIssue]
- [SelectGitHubIssue]
- [SelectIssueFromBranch]

[state]: ./state.md
[step]: ../config/step/step.md
[selectjiraissue]: ../config/step/SelectJiraIssue.md
[selectgithubissue]: ../config/step/SelectGitHubIssue.md
[selectissuefrombranch]: ../config/step/SelectIssueFromBranch.md
