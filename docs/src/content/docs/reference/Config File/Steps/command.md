---
title: Command
---

Run a command after optionally replacing some variables.
This step is here to cover the infinite things you might want to do that Knope doesn't yet know how to do itself.
If you have a lot of these steps or a complex `command`,
you might want to run a script in something like Bash or Python,
then call that script with a command.

## Example

If the current version for your project is `1.0.0`,
the following workflow step will run the `git` command with the arguments `tag` and `v.version`.

```toml
[[workflows.steps]]
type = "Command"
command = "git tag v.version"
variables = {"version" = "Version"}
```

## Variables

The `variables` attribute of this step is an object where the key is the string you wish to substitute
and the value is one of the [available variables](/reference/config-file/variables).
**Take care when selecting a key to replace** as Knope will replace _any_ matching string that it finds.
Replacements occur in the order they're declared in the config,
so Knope may replace earlier substitutions with later ones.

## Shell mode

By default, Knope splits commands into the executable name and its arguments, and calls the executable directly.
This works around common issues with Windows shells, particularly when quoting arguments.
However, you may want to use your current shell to run the command, for example to access environment variables or
to use shell features like pipes or redirection. You can do this by setting `shell=true` in the step configuration.

```toml
[[workflows.steps]]
type = "Command"
command = "echo $AN_ENV_VAR && echo $ANOTHER_ENV_VAR"
shell = true
```
