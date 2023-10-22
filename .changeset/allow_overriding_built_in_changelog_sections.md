---
default: minor
---

# Allow overriding built-in changelog sections

If you don't want to use the default changelog sections of "Breaking changes", "Features",
and "Fixes", you can now override them by using the equivalent changeset types!
Overriding them resets their position in the changelog, so you probably want to reset _all_ of them if you reset any.
This looks like:

```toml
[package]
extra_changelog_sections = [
    { type = "major", name = "❗️Breaking ❗" },
    { type = "minor", name = "🚀 Features" },
    { type = "patch", name = "🐛 Fixes" },
    { footer = "Changelog-Note", name = "📝 Notes" },
]
```
