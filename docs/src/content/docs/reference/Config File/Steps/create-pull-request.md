---
title: CreatePullRequest
---

Create a pull request on every configured forge from the current branch to a specified branch. If a pull request for those already exists, this step will overwrite the title and body of the existing pull request.

:::caution
If you configure more than one forge, Knope will assume that you wish to create a PR on all of them.
Keep that in mind, when writing your configuration.
:::

## Prerequisites

To use the `CreatePullRequest` step, you must configure a forge first. See [configuring a Forge] for more information.

## Parameters

### `base`

The branch that the pull request should target. This is a **required** parameter.

### `title.template`

A template string for the title of the pull request. This is a **required** parameter.

### `title.variables`

An optional map of variables to use in the title template.

### `body.template`

A template string for the body of the pull request. This is **required**.

### `body.variables`

An optional map of variables to use in the body template.

## Example

An example workflow which creates a pull request from the current branch to `main`.
This uses the current version of the package as the title and the changelog entry for the current version as the body:

```toml
[[workflows]]
name = "create-release-pull-request"

[[workflows.steps]]
type = "CreatePullRequest"
base = "main"

[workflows.steps.title]
template = "chore: Release $version"
variables = { "$version" = "Version" }

[workflows.steps.body]
template = "Merging this PR will release the following:\n\n$changelog"
variables = { "$changelog" = "ChangelogEntry" }
```

For a full example of how to use this with GitHub Actions to help automate releases, check out [Knope's prepare-release workflow] and [Knope's release workflow].

[Knope's prepare-release workflow]: https://github.com/knope-dev/knope/blob/e7292fa746fe1d81b84e5848815c02a0d8fc6f95/.github/workflows/prepare_release.yml
[knope's release workflow]: https://github.com/knope-dev/knope/blob/e7292fa746fe1d81b84e5848815c02a0d8fc6f95/.github/workflows/release.yml
[configuring a forge]: /reference/concepts/forge
