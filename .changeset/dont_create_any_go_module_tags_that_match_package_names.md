---
knope: major
---

# Don't create _any_ go module tags that match package names

Knope already avoided creating duplicate tags for Go modules which match tags that would be created by the `Release` step for the package.
Now, Knope won't create a Go module tag if it matches a release tag for _any_ configured package, to avoid potential conflicts.
