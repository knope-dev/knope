---
knope: minor
---

# Automatically release dependents when internal dependencies change

In a monorepo, setting `update_internal_dependencies = "patch"` (or `"minor"`) on a package now releases that package whenever one of its in-repo dependencies releases. The new release gets a changesets-style `Dependencies` section listing what triggered it, and propagation is transitive—releases are applied in dependency order.

Knope finds the relationships automatically: from each opted-in package's manifest (`Cargo.toml` or `package.json` dependencies), and from `versioned_files` entries that point at another package's files. For anything neither can see (for example, versions tracked only in a workspace-root manifest), declare the relationship explicitly with `internal_dependencies`:

```toml
[packages.consumer]
versioned_files = ["consumer/Cargo.toml"]
update_internal_dependencies = "patch"
internal_dependencies = ["my-lib"] # usually unnecessary
```

The default is `"none"`—nothing changes unless a package opts in. See the [`update_internal_dependencies` documentation](https://knope.tech/reference/config-file/packages#update_internal_dependencies) for details.
