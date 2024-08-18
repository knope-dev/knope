---
knope: major
---

# Auto-update Cargo workspace dependencies when using default config

If using the Cargo workspace [default configuration](https://knope.tech/reference/default-config/#cargo-workspaces),
Knope will now attempt to automatically update the version of workspace members in dependencies _and_ the workspace `Cargo.lock`.

To avoid this, use `knope --generate` to create a manual config file and customize the behavior.
