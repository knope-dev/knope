---
default: minor
---

#### Allow `Release` step to be in separate workflow as `PrepareRelease`

Previously, you needed to have a `PrepareRelease` step earlier in the same workflow if you wanted to use the `Release` step. Now, if you run a `Release` step _without_ a `PrepareRelease` step, Knope will check Git tags and versioned files to figure out if there's something new to release. This is especially useful if you want to build release assets using the new version (determined by `PrepareRelease`) before actually releasing them (using `Release`).
