# State

Throughout the course of a [workflow], Dobby will often have to remember some information from previous steps to use in future steps. This is done using an internal `State`. All workflows start in the [Initial] state and may transition between a states depending on the [steps] that are run. For example, the [SelectJiraIssue] step will transition the workflow to the [IssueSelected] state.

## Potential States

- [Initial]
- [IssueSelected]

[workflow]: ../config/workflow.md
[initial]: ./Initial.md
[issueselected]: ./IssueSelected.md
[steps]: ../config/step/step.md
[selectjiraissue]: ../config/step/SelectJiraIssue.md
