---
"versioning": major
---

Box `Version` fields in `NewError::InconsistentVersions` to reduce the error variant size and address the `result_large_err` clippy lint introduced in Rust 1.94.0. Callers that construct or pattern-match on `NewError::InconsistentVersions` must now use `Box<Version>` for the `first_version` and `second_version` fields.
