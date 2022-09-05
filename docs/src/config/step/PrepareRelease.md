# PrepareRelease step

This will look through all commits since the version tag and parse any [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) it finds. It will then bump the package version (depending on the [Semantic Versioning] rule determined from the commits) and add a new changelog entry using the [Keep A Changelog](https://keepachangelog.com/en/1.0.0/) format.

The version bumping follows the same rules and logic as the [BumpVersion] step, with the rule selected for you automatically. Which files are edited (both for versioning and changelog) is determined by the [packages] section.

When multiple [packages] are configured—`PrepareRelease` runs for each package independently. The version tag _for that package_ will be the starting point.

```admonish note
The last "version tag" is used as the starting point to read commits—that's the most recent tag that was created by the [`Release`] step. See that step for details on the tagging formats.
```

## Limitations

The CHANGELOG format is pretty strict. Only three sections will be added to the new version, `### Breaking Changes` for anything that conventional commits have marked as breaking, `### Fixes` for anything called `fix:`, and `### Features` for anything with `feat: `. Any other commits (conventional or not) will be left out.

## Commit Scopes

The `PrepareRelease` step can be fine-tuned when working with multiple packages to only apply a commit to a specific package's version & changelog. This is done by adding a `scopes` array to the [packages] config and adding a [conventional commit scope] to the commits that should not apply to all packages. The following rules apply, in order, with respect to conventional commit scopes:

1. If no packages define `scopes` in their config, all commits apply to all packages. Scopes are not considered by `knope`.
2. If a commit does not have a scope, it applies to all packages.
3. If a commit has a scope, and _any_ package has defined a `scopes` array, the commit will only apply to those packages which have that scope defined in their `scopes` array.

## Examples

### Creating a Pre-release Version

If you include the `prerelease_label` option, the version created will be a pre-release version (treated like `Pre` rule in [BumpVersion]). This allows you to collect the commits _so far_ to an impending future version to get them out earlier.

```toml
[package]
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

### Multiple Packages with Scopes

Here's a `knope` config with two packages: `cli` and `lib`.

```toml
[package.cli]
versioned_files = ["cli/Cargo.toml"]
changelog = "cli/CHANGELOG.md"
scopes = ["cli"]

[package.lib]
versioned_files = ["lib/Cargo.toml"]
changelog = "lib/CHANGELOG.md"
scopes = ["lib"]

[[workflows]]
name = "release"

[[workflows.steps]]
type = "PrepareRelease"
```

The `cli` package depends on the `lib` package, so they will likely change together. Let's say the version of `cli` is 1.0.0 and the version of `lib` is 0.8.9. We add the following commits:

1. `feat(cli): Add a new --help option to display usage and exit`
2. `feat(lib)!: Change the error type of the parse function`
3. `fix: Prevent a crash when parsing invalid input`

The first two commits are scoped—they will only apply to the packages which have those scopes defined in their `scopes` array. The third commit is not scoped, so it will apply to both packages.

```admonish note
Here, the configured scopes are the same a the name of the package. This is common, but not required.
```

When the `release` workflow is run, the `cli` package will be bumped to 1.1.0 and the `lib` package will be bumped to 0.9.0. The changelog for `cli` will look like this:

```md
## 1.1.0

### Features

- Add a new --help option to display usage and exit

### Fixes

- Prevent a crash when parsing invalid input
```

And the changelog for `lib` will look like this:

```md
## 0.9.0

### Breaking Changes

- Change the error type of the parse function

### Fixes

- Prevent a crash when parsing invalid input
```

## Errors

The reasons this can fail:

1. The version could not be bumped for some reason.
2. The [packages] section is not configured correctly.
3. There was nothing to release. In this case it exits immediately so that there aren't problems with later steps.

[semantic versioning]: https://semver.org
[bumpversion]: ./BumpVersion.md
[packages]: ../packages.md
[`release`]: ./Release.md
[conventional commit scope]: https://www.conventionalcommits.org/en/v1.0.0/#commit-message-with-scope
