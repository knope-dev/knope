---
title: "Default config"
---

Knope has some default configuration for things that aren't specified in a `knope.toml` file
(either when the file isn't present, or when there are missing sections).

To find out what _your project's_ default config is, use
`knope --generate` (with no `knope.toml` file in the current directory).

:::tip
Follow the [basics tutorial](/tutorials/releasing-basic-projects) to learn the default workflows hands-on.
:::

## Packages

When there are no packages defined in a `knope.toml` file, Knope tries to find supported packages automatically.

### Single package

Most of the time,
Knope will look for any [supported formats](/reference/config-file/packages#versioned_files) in the current directory
and add all of those to a single package:

```toml title="knope.toml"
[package]
versioned_files = ["Cargo.toml", "package.json"]
changelog = "CHANGELOG.md"
```

The `changelog` field is only set if there is a `CHANGELOG.md` file in the current directory.

### Cargo workspaces

If there is a `Cargo.toml` file in the current directory that looks like a Cargo workspace,
Knope will create a package for each member.

:::caution
Only a subset of Cargo workspace features are currently supported, notably members have to be explicitly listed, not using any `*`.
:::

```toml title="Cargo.toml"
[workspace]
members = ["member1", "member2"]
```

The names of these packages are from the `name` in their respective `Cargo.toml` files, not the directory name.
There _must_ be a `Cargo.toml` file in each member directory, or Knope will error.

```toml title="member1/Cargo.toml"
[package]
name = "something"
version = "0.1.0"
```

```toml title="member2/Cargo.toml"
[package]
name = "something-else"
version = "0.1.0"
```

```toml title="knope.toml"
[package.something]
versioned_files = ["member1/Cargo.toml"]
scopes = ["something"]

[package.something-else]
versioned_files = ["member2/Cargo.toml"]
scopes = ["something-else"]
```

## Workflows

When there are no workflows defined in a `knope.toml` file, Knope will use the default workflows.
Some pieces will differ depending on the configured packages and forges:

```toml title="knope.toml" {"Does not use a $version variable when there are multiple packages": 9-13} {"Moves git push down here and pushes tags if no forges are configured": 18}
[[workflows]]
name = "release"

[[workflows.steps]]
type = "PrepareRelease"

[[workflows.steps]]
type = "Command"
command = "git commit -m \"chore: prepare release $version\""

[[workflows.steps]]
type = "Command"
command = "git push"

[workflows.steps.variables]
"$version" = "Version"

[[workflows.steps]]
type = "Release"



[[workflows]]
name = "document-change"

[[workflows.steps]]
type = "CreateChangeFile"
```

## Forges

If there is a `knope.toml` file, **no forges will be configured by default**.
If there is _no_ `knope.toml` file, Knope will look at the first Git remote to determine a forge config.

For example, if the first Git remote is `git@github.com:knope-dev/knope.git`, Knope will generate a GitHub config:

```toml title="knope.toml"
[github]
owner = "knope-dev"
repo = "knope"
```

See [forges](/reference/concepts/forge) for more info.
