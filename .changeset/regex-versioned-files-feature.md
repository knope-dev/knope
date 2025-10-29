---
knope: minor
---

Add support for updating version numbers in arbitrary text files using regex patterns. You can now specify versioned files with a `regex` field containing a named capture group called `version`:

```toml
[package]
versioned_files = [
    "Cargo.toml",
    { path = "README.md", regex = "version:\\s+(?<version>\\d+\\.\\d+\\.\\d+)" }
]
```

This allows Knope to automatically update version numbers in documentation, installation instructions, and other text files that don't have a structured format.
