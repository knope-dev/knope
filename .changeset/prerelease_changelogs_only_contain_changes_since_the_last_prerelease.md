---
knope: major
versioning: major
---

# Prerelease changelogs only contain changes since the last prerelease

When running `PrepareRelease` with a prerelease label, Knope now collects conventional commits since the most recent tag of _any_ kind—including prereleases—instead of the last stable release. Iterative prerelease workflows (`alpha.1` → `alpha.2`) get changelogs containing only the commits added since the previous prerelease, and packages with no new commits are no longer re-released on every run.

If the commits since the last prerelease imply a lower version than an existing prerelease line (for example, a fix following a feature that already shipped in `1.1.0-rc.0`), the new version continues the existing line (`1.1.0-rc.1`) rather than regressing below a version that has already been pre-released. This also applies to `knope-versioning`'s version calculation: a rule-derived version with no prerelease line of its own continues the nearest existing line above it.

Stable releases are unaffected: they still summarize everything since the last stable release.
