---
default: major
---

#### Prevent bumping major version of a `go.mod` file

According to [the docs](https://go.dev/blog/v2-go-modules), aside from the `v0` -> `v1` transition, `go.mod` files should not be updated for new major versions, but instead a new `v{major}` directory should be created with a new `go.mod` file. This is for compatibility with older versions of Go tools.

In order to prevent someone from accidentally doing the wrong thing, Knope will no longer bump a `go.mod` file to `v2` unless `--override-version` is used to bypass this check. Additionally, if a `go.mod` file is in the matching versioned directory (e.g., the `go.mod` file ending in `/v2` is under a top-level directory called `v2`), Knope will not allow the major version of _that_ file to be bumped, as it would break the package.
