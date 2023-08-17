---
default: patch
---

#### Use the correct tags for `go.mod` files in subdirectories

PR #544 fixed issue #502 by @BatmanAoD.

Previously, the version for a `go.mod` file was determined by the package tag, named `v{Version}` for single packages or `{PackageName}/v{Version}` for named packages. This worked when the `go.mod` file was in the root of the repository or a directory named `{PackageName}` (respectively), but not when it was in a different directory. Now, the version tag, both for determining the current version and creating a new release, will correctly be determined by the name of the directory the `go.mod` file is in (relative to the working directory). The existing package tagging strategy remains unchanged.

For example, consider this `knope.toml` file:

```toml
[package]
versioned_files = ["some_dir/go.mod"]
```

Previous to this release, creating the version `1.2.3` would only create a tag `v1.2.3`. Now, it will _additionally_ create the tag `some_dir/v1.2.3`.
