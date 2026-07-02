---
title: Updating dependencies
---

When you have multiple packages which depend on each other, you'll likely want to keep those dependencies up to date.

:::caution

Not all versioned files support updating dependencies,
so be sure to check the [reference](/reference/config-file/packages#versioned_files) for more information, and
[open an issue](https://github.com/knope-dev/knope/issues/new) if you need support for a new file type.

:::

For example, Knope depends on the `knope-versioning` crate.
Whenever `knope-versioning` is updated, the new version should appear in:

1. `knope-versioning`'s `Cargo.toml`, defining the new version of the package
2. `knope`'s `Cargo.toml`, specifying the new version of `knope-versioning` as a dependency
3. The workspace `Cargo.lock`, recording the exact version of `knope-versioning` used by the workspace

:::note

Knope can currently only update dependencies to _exact versions_, it won't retain partial semantic versions or ranges.

:::

Before adding dependency updates, the relevant package config looks like this:

```toml title="knope.toml"
[packages.versioning]
versioned_files = ["crates/knope-versioning/Cargo.toml"]
```

:::tip

You can also specify a package name explicitly using the `name` field of each dependency.

:::

The lock file can be added as a normal path. Because it has a specific file name, Knope knows what
[file type](/reference/config-file/packages#versioned_files) it is.

```toml title="knope.toml" ""Cargo.lock""
[packages.versioning]
versioned_files = ["crates/knope-versioning/Cargo.toml", "Cargo.lock"]
```

Knope will use the `package.name` field from the first `Cargo.toml` file to determine which package to update in `Cargo.lock`.

For the other `Cargo.toml` file, you must specify that a dependency _within_ the file is what should be versioned,
not the package itself:

```toml title="knope.toml"
[packages.versioning]
versioned_files = [
    "crates/knope-versioning/Cargo.toml",
    "Cargo.lock",
    { path = "crates/knope/Cargo.toml", dependency = "knope-versioning" },
]
```

:::tip

You can use this same `dependency` field to override the name of a package updated in a `Cargo.lock`.

:::

## Releasing dependents automatically

Wiring up `versioned_files` keeps the dependency _string_ inside dependent packages in sync,
but by default each dependent must still have its own [Changeset](/reference/concepts/change)
to actually trigger a release of that dependent.

To release dependents automatically whenever one of their internal dependencies releases, set
`update_internal_dependencies` on the dependent's package config to `"patch"` or `"minor"`.
See the
[`update_internal_dependencies` reference](/reference/config-file/packages#update_internal_dependencies)
for details. The default is `"none"`: Knope keeps dependency strings in sync but creates no
release for the dependent.

For example, take the earlier setup where `knope` depends on `knope-versioning`. Set
`update_internal_dependencies = "patch"` on the `knope` package. Now releasing
`knope-versioning` also releases `knope` as a patch, and the new `knope` release notes
include a `Dependencies` section that lists the bumped dependencies.

:::caution

Knope finds the dependency relationships by reading each opted-in package's manifests
(`Cargo.toml`, `package.json`) and from `versioned_files` entries that point at another
package's files. Sometimes a relationship isn't visible to either. For example, you might
track the version only in the workspace-root `Cargo.toml` via `[workspace.dependencies]`. In
those cases, declare it explicitly with
[`internal_dependencies`](/reference/config-file/packages#internal_dependencies) on the
dependent's package config.

:::
