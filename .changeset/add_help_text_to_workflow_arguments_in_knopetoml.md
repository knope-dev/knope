---
default: minor
---

# Add `help_text` option to workflows

`[[workflows]]` can now have `help_text`:

Example:

```toml
[[workflows]]
name = "release"
help_text = "Prepare a release"
```

The message is displayed when running `knope --help`:

```text
A command line tool for automating common development tasks

Usage: knope [OPTIONS] [COMMAND]

Commands:
  release          Prepare a release
  help             Print this message or the help of the given subcommand(s)

...
```

PR #960 closes issue #959. Thanks @alex-way!
