---
knope: minor
config: minor
---

# Add `[changes]` config section with `ignore_conventional_commits` setting

Adds a new top-level `[changes]` configuration section to control how Knope processes changes. The first setting in this section is `ignore_conventional_commits`, which when set to `true`, makes Knope ignore conventional commits and only use changesets for determining version bumps and changelog entries.

This replaces the deprecated step-level `ignore_conventional_commits` option on the `PrepareRelease` step. Use `knope --upgrade` to automatically migrate from the old format to the new one.

**Example configuration:**

```toml
[changes]
ignore_conventional_commits = true

[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"
```

See the [changes config documentation](https://knope.tech/reference/config-file/changes) for more details.
