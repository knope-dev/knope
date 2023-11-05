---
title: "Default workflows"
---

When there is no `knope.toml` file in the current directory, Knope will use a set of default workflows.
These will vary slightly depending on your project,
the easiest way to find out what your default workflows are is to use `knope --generate`.

:::tip
Follow the [basics tutorial](/tutorials/releasing-basic-projects) to learn the default workflows hands-on.
:::

## General structure

The default workflows are as follows, with potential differences highlighted:

```toml title="knope.toml" {2,3,18-19,27-29}
[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"

[[workflows]]
name = "release"

[[workflows.steps]]
type = "PrepareRelease"

[[workflows.steps]]
type = "Command"
command = "git commit -m \"chore: prepare release $version\" && git push"

[workflows.steps.variables]
"$version" = "Version"

[[workflows.steps]]
type = "Release"

[[workflows]]
name = "document-change"

[[workflows.steps]]
type = "CreateChangeFile"

[github]
owner = "knope-dev"
repo = "knope"
```

## Potential differences

1. `versioned_files` contains any [supported formats](/reference/config-file/packages#versioned_files) that are detected in the current directory
2. `changelog` will not be populated if there is not a `CHANGELOG.md` file in the current directory
3. A [GitHub config](/reference/config-file/github) will be set if the default Git remote is a GitHub repository.
   Otherwise, an additional step will be added to the `release` workflow to push generated tags.
