---
default: major
---

#### Workflows can no longer be selected interactively

Previously, it was valid to invoke `knope` with no arguments, and the user would be prompted interactively to select a workflow. Now, a workflow must be provided as a positional argument, for example, `knope release`.
