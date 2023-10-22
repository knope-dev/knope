---
default: major
---

# Change where new versions are inserted in changelog

In practice, this will not impact most changelogs, however, previous versions of Knope looked for the first header at a certain level (e.g., starting with `## `) and inserted the new version right before that. Now, Knope looks for the first header that is parseable as a semver version (e.g., `## 1.2.3`) and inserts the new version right before that.

This _will_ make it harder to adopt Knope in projects that have an existing changelog which is not of the same format,
but it makes inserting the new version in the changelog more robust.
