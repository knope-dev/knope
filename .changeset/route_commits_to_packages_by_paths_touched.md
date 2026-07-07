---
knope: minor
---

# Route commits to packages by the paths they touch with `track_paths`

Setting `track_paths = true` on a package routes conventional commits to it based on the files each commit changed, instead of relying on commit scopes. By default, a package's territory is the parent directories of its own `versioned_files`; set `paths` to override it:

```toml
[packages.my-package]
versioned_files = ["my-package/Cargo.toml"]
track_paths = true
paths = ["my-package", "docs/my-package"] # optional override
```

Commits that touch multiple packages' territories apply to each of them, and commits touching no tracked territory are ignored. See the [`track_paths` documentation](https://knope.tech/reference/config-file/packages#track_paths) for details.
