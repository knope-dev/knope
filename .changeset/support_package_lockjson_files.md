---
versioning: minor
knope: major
---

# Support `package-lock.json` files

`package-lock.json` files are [now supported](https://knope.tech/reference/config-file/packages/#package-lockjson)
as `versioned_files` both for single packages and dependencies (in monorepos).

These files will be auto-detected and updated if using the default (no `[package]` or `[packages]`) config, so
this is a breaking change for those users.
