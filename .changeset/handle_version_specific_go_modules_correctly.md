---
default: patch
---

#### Handle version-specific go modules correctly

Fixes #584 from @BatmanAoD.

If you have a `go.mod` file representing a specific major version in a directory (as recommended in [the go docs](https://go.dev/blog/v2-go-modules)), Knope will now tag it correctly. Previously, a `v2/go.mod` file would generate a tag like `v2/v2.1.3`. Now, it will generate a tag like `v2.1.3`.

Additionally, when determining the _current_ version for a `go.mod` file, only tags which match the major version of the `go.mod` file will be considered.
