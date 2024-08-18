---
knope: patch
---

# Deduplicate release actions

Knope now collects all actions to be performed across all packages and runs them at once with deduplication.

This means that if multiple packages write to the same `versioned_file`, for example, the file will only be written 
a single time.
Changesets will also only be deleted once, files will be staged to Git only once, etc.

This mostly only impacts the output during `--dry-run` or `--verbose`, but is especially important for the new 
dependency updating and lockfile support.
