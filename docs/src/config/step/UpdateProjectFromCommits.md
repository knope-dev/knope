# UpdateProjectFromCommits step

This will look through all commits since the last tag and parse any [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) it finds. It will then bump the project version (depending on the [Semantic Versioning] rule determined from the commits) and add a new Changelog entry using the [Keep A Changelog](https://keepachangelog.com/en/1.0.0/) format.

## Limitations

The CHANGELOG format is pretty strict, it needs to have at least one version already in it and every version needs to be a level 2 header (`## 1.0.0`). Only three sections will be added to the new version, `### Breaking Changes` for anything that conventional commits have marked as breaking, `### Fixes` for anything called `fix:`, and `### Features` for anything with `feat: `. Any other commits (conventional or not) will be left out. A new version will **always** be generated though, even if there are no changes to record.

## Examples

You can either provide an explicit path when declaring the step, like this:

```toml
[[workflows]]
name = "Release"

    [[workflows.steps]]
    type = "UpdateProjectFromCommits"
    changelog_path = "docs/CHANGELOG.md"
```

or omit the `changelog_path`, which will default it to "CHANGELOG.md" in the current directory.

## Errors

The reasons this can fail:

1. If there is no previous tag to base changes off of.
1. The provided path to the changelog file could not be found.
1. The version could not be bumped for some reason.

[semantic versioning]: https://semver.org
