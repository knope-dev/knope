---
default: minor
---

#### Add a `ChangelogEntry` variable for substitution

Anywhere that the existing `Version` variable can be used (for example, in [the `Command` step]), you can now also use `ChangelogEntry` to get the section of the changelog that corresponds to the current version. For example, you could (almost) replicate Knope's GitHub Release creation _without_ Knope's GitHub integration with a workflow like this:

```toml
[[workflows]]
name = "release"

[[workflows.steps]]
type = "PrepareRelease"

[[workflows.steps]]
type = "Command"
command = "git commit -m \"chore: prepare release $version\" && git push"

[workflows.steps.variables]
"$version" = "Version"

[[workflows.steps]]
type = "Command"
command = "gh release create --title '$version' --notes '$changelog'"

[workflows.steps.variables]
"$version" = "Version"
"$changelog" = "ChangelogEntry"
```

[the `Command` step]: https://knope-dev.github.io/knope/config/step/Command.html
