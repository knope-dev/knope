---
default: minor
---

# Add basic Cargo workspace support

If you have a `Cargo.toml` file in the working directory which represents a Cargo workspace containing fixed members, like:

```toml
[workspace]
members = [
  "my-package",
  "my-other-package",
]
```

then Knope will now treat each member like a package, assuming that there is a `Cargo.toml` file for each member.

This does not work with path globbing yet, only manual directory entries. See [the new docs](https://knope.tech/reference/default-config/#cargo-workspaces) for more details.
