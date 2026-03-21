---
versioning: major
---

# `NewError::InconsistentVersions` fields are now `Box<Version>`

The `first_version` and `second_version` fields of `NewError::InconsistentVersions` (re-exported as `PackageNewError`) changed from `Version` to `Box<Version>` to reduce the variant's size and satisfy the `clippy::result_large_err` lint introduced in Rust 1.94.0.

Callers that construct or pattern-match on this variant must now wrap/unwrap accordingly:

```rust
// Before
NewError::InconsistentVersions { first_version, .. } => println!("{first_version}"),

// After
NewError::InconsistentVersions { first_version, .. } => println!("{first_version}"), // Box<T: Display> derefs automatically

// Constructing
NewError::InconsistentVersions {
    first_version: Box::new(version),
    ..
}
```
