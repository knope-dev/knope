---
title: RebaseBranch
---

Rebase the current branch onto the branch defined by `to`.

## Errors

Fails if any of the following are true:

1. The current directory isn't a Git repository.
2. Knope can't find the `to` branch locally (doesn't check remotes).
3. The repo isn't on the tip of a branch (for example, detached `HEAD`)
4. Rebase fails (for example, not a clean working tree)

## Example

```toml
[[workflows]]
name = "Finish some work"
    [[workflows.steps]]
    type = "RebaseBranch"
    to = "main"
```
