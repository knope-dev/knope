# PrepareRelease step

This will look through all commits since the last tag and parse any [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) it finds. It will then bump the project version (depending on the [Semantic Versioning] rule determined from the commits) and add a new Changelog entry using the [Keep A Changelog](https://keepachangelog.com/en/1.0.0/) format.

The version bumping follows the same rules and logic as the [BumpVersion] step, with the rule selected for you automatically. Which files are edited (both for versioning and changelog) is determined by the [`packages`] section.

```admonish note
While the most recent tag is used to select commits, the current version is determined by the `versioned_files` in the [`packages`] section. The content of the tag is irrelevant.
```

## Limitations

The CHANGELOG format is pretty strict, it needs to have at least one version already in it and every version needs to be a level 2 header (`## 1.0.0`). Only three sections will be added to the new version, `### Breaking Changes` for anything that conventional commits have marked as breaking, `### Fixes` for anything called `fix:`, and `### Features` for anything with `feat: `. Any other commits (conventional or not) will be left out. A new version will **always** be generated though, even if there are no changes to record.

## Examples

### Creating a Pre-release Version

If you include the `prerelease_label` option, the version created will be a pre-release version (treated like `Pre` rule in [BumpVersion]). This allows you to collect the commits _so far_ to an impending future version to get them out earlier.

```toml
[[packages]]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"

[[workflows]]
name = "prerelease"

[[workflows.steps]]
type = "PrepareRelease"
prerelease_label = "rc"
```

Note that after you've done this, the final release created later will not include change notes from the intermediate pre-release versions.

## Errors

The reasons this can fail:

1. If there is no previous tag to base changes off of.
2. The version could not be bumped for some reason.
3. The [`packages`] section is not configured correctly.

[semantic versioning]: https://semver.org
[bumpversion]: ./BumpVersion.md
[`packages`]: ../packages.md
