---
default: minor
---

# Add help_text to workflow arguments in knope.toml

This change allows users to specify a `help_text` for a workflow in their `knope.toml` file within the `[[workflows]]` section.

Example:

```toml
[[workflows]]
name = "release"
help_text = "Prepare a release"
```

When running `knope --help`, this will be displayed:

```text
A command line tool for automating common development tasks

Usage: knope [OPTIONS] [COMMAND]

Commands:
  prepare-release
  release          Prepare a release
  document-change
  pwd
  help             Print this message or the help of the given subcommand(s)

Options:
      --dry-run   Pretend to run a workflow, outputting what _would_ happen without actually doing it.
  -v, --verbose   Print extra information (for debugging)
      --upgrade   Upgrade to the latest `knope.toml` syntax from any deprecated (but still supported) syntax.
      --validate  Check that the `knope.toml` file is valid.
  -h, --help      Print help
  -V, --version   Print version
```
