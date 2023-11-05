---
title: Multiple versioned files
---

Sometimes you have a [package](/reference/concepts/package)
that has multiple files which need to be versioned together.
This is why the `versioned_files` attribute of the `[package]` section in config is an array.

:::caution
A package only has one version, so every `versioned_file` must have the same version.
If there are multiple versions to track, these are separate packages,
and you should check out the [releasing multiple packages tutorial](/tutorials/releasing-multiple-packages).
:::

Start by creating a `knope.toml` file if you don't already have one:

```sh
knope --generate
```

Then, edit the `versioned_files` attribute of the `[package]` section to add additional versioned files:

```toml
[package]
versioned_files = ["Cargo.toml", "a_dir/package.json", "some_python/pyproject.toml", "and_also/go.mod"]
```

:::note
All the possible formats for versioned files are listed in the [reference](/reference/config-file/packages#versioned_files).
:::

Now, releases will update the version of all those files together.
