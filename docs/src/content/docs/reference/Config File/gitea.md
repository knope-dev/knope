---
title: "Gitea"
---

Details needed to use steps that reference Gitea repositories.

## Example

```toml
# knope.toml

[gitea]
owner = "knope-dev"
repo = "knope"
host = "codeberg.org"
```

The first time you use a step which requires this config,
you will be prompted to generate a Gitea API token so Knope can perform actions on your behalf.
To bypass this prompt, you can manually set the `GITEA_TOKEN` environment variable.
