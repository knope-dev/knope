---
default: minor
---

# Use default packages/workflows even when `knope.toml` exists

If you define a `knope.toml` file without any packages, Knope will assume the default packages (as if you had no `knope.toml` file at all).

Likewise, if you have no `[[workflows]]` in a `knope.toml` file, Knope will assume the default workflows.
