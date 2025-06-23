---
knope: patch
versioning: patch
---

# Fix multiple versioned files with same path

Previously, if you referenced the same file multiple times in versioned_files, only the first instance would apply.
Now, if you reference the same path multiple times, each instance will be processed sequentially.

Consider a mono-repo where every package should be versioned in lockstep:

```toml
[package]
versioned_files = [
  "package.json",
  { path = "package.json", dependency = "aDependency" },  # Before this fix, this was ignored
  "aDependency/package.json"
]
```

This use-case is now supported!
