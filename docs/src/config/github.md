# GitHub

Details needed to use steps that reference GitHub repos.

## Example

```TOML
# knope.toml

[github]
owner = "knope-dev"
repo = "knope"
```

The first time you use a step which requires this config, you will be prompted to generate a GitHub API token so knope can perform actions on you behalf. To bypass this prompt, you can manually set the `GITHUB_TOKEN` environment variable.
