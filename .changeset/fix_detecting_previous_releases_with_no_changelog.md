---
knope: patch
---

# Fix detecting previous releases with no changelog

Previously, if you ran a `PrepareRelease` step with no `changelog` to modify in one workflow and then a `Release` step 
in a separate workflow, `Release` would fail to create a Git tag.

Now, a release with "no notes" will properly be created if the last Git tag doesn't match the current version of 
a file.
If a forge is configured, a release will be created on that forge without any notes,
and a title simply containing the version.

Fixes #1267
