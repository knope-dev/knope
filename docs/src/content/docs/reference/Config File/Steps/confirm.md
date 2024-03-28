---
title: Confirm
---

Prompt the user to confirm a given message. Approving the prompt will continue to run the workflow.
Rejecting the prompt will stop the workflow and no further steps will be run.

## Example

```toml
[[workflows.steps]]
type = "Confirm"
message = "Are you sure you want to run the cleanup step?"
```

The example workflow above will promp the user with the following message:

```shell

? Are you sure you want to run the cleanup step? (Y/n)

```

## Automatic confirmation

If you want to automatically confirm all steps in a workflow, you can run the workflow with either the `--assumeyes` or `-y` flag.
