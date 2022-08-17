# PrepareRelease step

This will look through all commits since the version tag and parse any [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) it finds. It will then bump the project version (depending on the [Semantic Versioning] rule determined from the commits) and add a new Changelog entry using the [Keep A Changelog](https://keepachangelog.com/en/1.0.0/) format.

The version bumping follows the same rules and logic as the [BumpVersion] step, with the rule selected for you automatically. Which files are edited (both for versioning and changelog) is determined by the [`packages`] section.

```admonish note
The last "version tag" is used as the starting point to read commits—that's the most recent tag that looks like v<semantic_version>. v1.2.3 and v1.2.3-rc.1 are both valid version tags. However, if you are releasing a non-pre version (no `prerelease_label` is set for the step), prerelease tags are ignored. See examples below for more detail.
```

## Limitations

The CHANGELOG format is pretty strict. Only three sections will be added to the new version, `### Breaking Changes` for anything that conventional commits have marked as breaking, `### Fixes` for anything called `fix:`, and `### Features` for anything with `feat: `. Any other commits (conventional or not) will be left out. A new version will **always** be generated though, even if there are no changes to record.

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

If your prerelease workflow is exactly like your release workflow, you can instead temporarily add a prerelease label by passing the `--prerelease-label` option to `knope` or by setting the `KNOPE_PRERELEASE_LABEL` environment variable. This option overrides any set `prerelease_label` for any workflow run.

### Going from Pre-release to Full Release

Let's say that in addition to the configuration from the above example, you also have a section like this:

```toml
[[workflows]]
name = "release"

[[workflows.steps]]
type = "PrepareRelease"
```

And your changelog looks like this (describing some pre-releases you already have):

```md
## 2.0.0-rc.1

### Bug Fixes

- A bug in the first `rc` that we fixed.

## 2.0.0-rc.0

### Breaking Changes

- Cool new API

## 1.14.0

The last 1.x release.
```

Now you're ready to release 2.0.0—the version that's going to come after 2.0.0-rc.1. If you run the defined `release` rule, it will go all the way back to the tag `v1.14.0` and use the commits from that point to create the new version. In the end, you'll get version 2.0.0 with a new changelog entry like this:

```md
## 2.0.0

### Breaking Changes

- Cool new API

### Bug Fixes

- A bug in the first `rc` that we fixed.
```

## Errors

The reasons this can fail:

1. If there is no previous tag to base changes off of.
2. The version could not be bumped for some reason.
3. The [`packages`] section is not configured correctly.

[semantic versioning]: https://semver.org
[bumpversion]: ./BumpVersion.md
[`packages`]: ../packages.md
