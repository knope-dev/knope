---
versioning: minor
knope: minor
---

# Add support for dependencies in `package.json` files

You can now update the `dependencies` and `devDependencies` of a `package.json` like this:

```toml
[package]
versioned_files = [
  # Update @my/package-name in dependencies and devDependencies whenever this package version updates
  { path = "package.json", dependency = "@my/package-name" }
]
```
