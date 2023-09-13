---
default: major
---

### Ignore unreachable tags when determining version

PR #574 fixes issue #505 from @BatmanAoD.

Previously, the latests tags were always used to determine the current version, **even if those tags were not reachable from `HEAD`**. Now, only reachable tags will be considered. Use the `--verbose` flag to see tags which are being ignored.
