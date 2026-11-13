---
knope: major
versioning: major
---

# Support quotes in changeset package names

In changesets, surrounding a package name with quotes no longer includes the quotes in the package name.
So all of these values in a changeset frontmatter are now equivalent:

- `my-package: minor`
- `"my-package": minor`
- `'my-package': minor`

This matches the behavior of the JavaScript `changeset` package.
