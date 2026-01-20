---
versioning: major
---

# Change several versioning APIs to encapsulate update behavior better

Notably `Package.versions` is no longer `pub` and instead there are new functions for interacting with it:

- `set_version` which replaces `bump_version`
- `latest_version` instead of `Package.versions.into_latest()`
- Non-mutating `calculate_new_version` instead of `Package.versions.bump`
