---
knope: major
---

# Change to default handling of top-level `package.json` files

When using the default config (no `[package]` or `[packages]`), Knope will now treat a top-level `package.json` file
which contains a `workspaces` property as the entrypoint into a monorepo and _not_ a single versioned_file package.
