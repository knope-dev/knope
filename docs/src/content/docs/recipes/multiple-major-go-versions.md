---
title: Multiple major Go versions
---

The [recommended best practice](https://go.dev/blog/v2-go-modules) for maintaining multiple major versions of Go modules is to include every major version on your main branch (rather than separate branches). To support multiple go modules files in Knope, you have to define them as separate packages:

```toml
# knope.toml
[packages.v1]
versioned_files = ["go.mod"]
scopes = ["v1"]

[packages.v2]
versioned_files = ["v2/go.mod"]
scopes = ["v2"]
```

This way, you can add features or patches to just the major version that a commit affects and release new versions of each major version independently.

:::danger
If you use this multi-package syntax for go modules, you **can't** use Knope to increment the major version. You'll have to create the new major version directory yourself and add a new package to `knope.toml` for it.
:::
