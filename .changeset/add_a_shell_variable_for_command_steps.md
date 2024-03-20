---
default: minor
---

# Add a `shell` variable for `Command` steps

You can now add `shell=true` to a `Command` step to run the command in the current shell.
This lets you opt in to the pre-0.15.0 behavior.

```toml
[[workflows.steps]]
type = "Command"
command = "echo $AN_ENV_VAR"
shell = true
```
