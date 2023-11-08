---
title: "Workflow"
---

A task that Knope can perform, triggered by passing a positional argument to Knope.
`knope release` runs the `release` workflow.

Every workflow has a series of [steps](/reference/concepts/step).

If there is a `knope.toml` file in the current directory, it defines every workflow.
If not, only the [default workflows](/reference/default-workflows) are available.
