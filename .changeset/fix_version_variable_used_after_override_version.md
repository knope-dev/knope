---
knope: patch
---

# Fix `$version` variable used after `--override-version`

Previously, if you used `BumpVersion` or `PrepareRelease` with `--override-version` and _then_ used a `$version` variable with
a `Command`, the variable would still be set to the original version (pre-bump).

Thanks @andrewmcgivery for the report!
