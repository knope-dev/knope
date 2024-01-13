---
default: minor
---

# Gitea support

- Added Support for Gitea in the `CreatePullRequest` step
- Added Support for Gitea in the `Release` step
- Added A new `SelectGiteaIssue` step
- Add support to generate Gitea config from known public Gitea instances

## How it works

To be able to use these new steps, just add a new section to your configuration, like this:

```toml
[gitea]
repo = "knope"
owner = "knope-dev"
host = "https://codeberg.org"
```

You can now use the supported steps in the same way as their GitHub equivalents.

## Generating a configuration

Knope can now generate a configuration for you, if your repository's remote is one of the known 
public Gitea instances. Currently only [Codeberg](https://codeberg.org) is supported, 
but feel free to add more [here](https://github.com/knope-dev/knope/blob/main/src/config/toml/config.rs#L90).
