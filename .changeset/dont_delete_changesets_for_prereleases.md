---
default: major
---

# Don't delete changesets for prereleases

Previously, using `PrepareRelease` to create a prerelease (for example, with `--prerelease-label`) would delete all
changesets, just like a full release. This was a bug, but the fix is a breaking change if you were
relying on that behavior.
