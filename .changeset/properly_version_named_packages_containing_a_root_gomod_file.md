---
default: patch
---

#### Properly version named packages containing a root `go.mod` file

Consider this package config in a `knope.toml`:

```toml
[packages.something]
versioned_files = ["go.mod"]
```

The `Release` step previously (and will still) add a tag like `something/v1.2.3`, however the correct Go module tag is `v1.2.3` (without the package name prefix). Knope will now correctly add this second tag (previously, top-level tags were only added for single-package repos).
