---
title: "Glossary"
description: "Every relatively-obscure term used in these docs is defined here."
---

## Workflow

A task that Knope can perform, triggered by passing a positional argument to Knope.
`knope release` runs the `release` workflow.

Every workflow is composed of a series of [steps](#step).

If there is a `knope.toml` file in the current directory, every workflow is defined there.
If not, only the [default workflows](/reference/default_workflows) are defined.

## Step

An atomic piece of a workflow. Every possible step is defined in [the config reference](/reference/config/steps).
