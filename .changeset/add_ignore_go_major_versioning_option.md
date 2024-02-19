---
default: minor
---

# Add `ignore_go_major_versioning` option

You can now set `ignore_go_major_versioning = true` for a package in
`knope.toml` to turn off the major version validation & updating in `go.mod` files.

More details in [the new docs](https://knope.tech/reference/config-file/packages/#ignore_go_major_versioning).

Closes #863, thanks for the suggestion @BatmanAoD!
