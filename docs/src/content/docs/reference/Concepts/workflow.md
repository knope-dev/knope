---
title: "Workflow"
---

A task that Knope can perform, triggered by passing a positional argument to Knope.
`knope release` runs the `release` workflow.

Every workflow is composed of a series of [steps](/reference/concepts/step).

If there is a `knope.toml` file in the current directory, every workflow is defined there.
If not, only the [default workflows](/reference/default-workflows) are defined.
