---
default: major
---

# `Cargo.toml` files must now have a `package.name` property

This was already required by Cargo, but wasn't enforced by Knope until now. Before, a `Cargo.toml` file like

```toml
[package]
version = "0.1.0"
```

was acceptable, but now it must be

```toml
[package]
name = "my-package"
version = "0.1.0"
```
