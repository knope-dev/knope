---
versioning: major
---

# `NewError::InconsistentVersions` fields are now `Box<Version>`

The `first_version` and `second_version` fields of `NewError::InconsistentVersions` (re-exported as `PackageNewError`) changed from `Version` to `Box<Version>` to reduce the variant's size and satisfy the `clippy::result_large_err` lint introduced in Rust 1.94.0.
