# RebaseBranch step

Rebase the current branch onto the branch defined by `to`.

## Errors

Fails if any of the following are true:

1. The current directory is not a Git repository.
2. The `to` branch cannot be found locally (does not check remotes).
3. The repo is not on the tip of a branch (e.g. detached HEAD)
4. Rebase fails (e.g. not a clean working tree)

## Example

```toml
[[workflows]]
name = "Finish some work"
    [[workflows.steps]]
    type = "RebaseBranch"
    to = "main"
```
