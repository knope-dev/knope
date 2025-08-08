---
versioning: major
---

# `ReleaseNotes::create_release` is no longer `pub`

This wasn't being used by Knope or Knope Bot directly, and non-pub functions give better lints.
If anyone is using this crate and needed that function, let me know!
