---
title: Command
---

Run a command in your current shell after optionally replacing some variables.
This step is here to cover the infinite things you might want to do that Knope doesn't yet know how to do itself.
If you have a lot of these steps or a complex `command`,
you might want to run a script in something like Bash or Python,
then call that script with a command.

## Example

If the current version for your project is `1.0.0`,
the following workflow step will run `git tag v.1.0.0` in your current shell.

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

## Working directory

By default, the command will be run from the current working directory.
If you want to run the command from the directory of the first config file in the ancestry of the current working directory,
you can set the `use_working_directory` attribute to `false`.
