---
title: Workflow
---

A workflow is a command that Knope can perform.
Every workflow is defined in the top-level `workflows` array of `knope.toml`.
The `name` field is a required string which is how the workflow will be referenced.
The `steps` array is an array of steps, each is unique, see "Steps" in the nav for details.

## Example

```toml
# knope.toml

[[workflows]]
name = "release"
    [[workflows.steps]]
    # First step details here
    [[workflows.steps]]
    # second step details here
```

This workflow would be executed like `knope release`.
