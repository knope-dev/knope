---
knope: minor
---

# Support glob patterns for package assets

You can now provide a glob pattern when defining package assets instead of specifying each file individually in a list.
This is especially useful when your asset names are dynamic (containing a version, date, or hash, for example) or
when different releases have different assets.

Knope will _not_ error if the glob pattern doesn't match any files.
You can't combine glob patterns with individual file names.

```toml
[package]
assets = "assets/*"
```
