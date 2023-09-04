---
default: minor
---

#### Upload assets to GitHub Releases

You can now add assets to a package like this:

```toml
[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"

[[package.assets]]
path = "artifact/knope-x86_64-unknown-linux-musl.tgz"
name = "knope-x86_64-unknown-linux-musl.tgz"

[[package.assets]]
path = "artifact/knope-x86_64-pc-windows-msvc.tgz"
name = "knope-x86_64-pc-windows-msvc.tgz"
```

When running the `Release` step with a valid `[github]` config, instead of immediately creating the release, Knope will:

1. Create a draft release
2. Upload all listed assets (erroring if any don't exist)
3. Publish the release
