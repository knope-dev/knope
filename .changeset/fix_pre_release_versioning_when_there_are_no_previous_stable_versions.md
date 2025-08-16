---
versioning: patch
knope: patch
---

# Fix pre-release versioning when there are no previous stable versions

Previously, if there was not a Git tag containing a previous stable version, Knope would default to "0.0.0".
Because of the special 1.0.0 rules, this also meant there was _no_ way to start a project at a pre-release of 1.0.0
with no prior releases.

Now, if there are no previous stable releases, Knope will use the version in your files instead of calculating a version
based on "0.0.0".

Fixes #1515, thanks for the report @hazre!
