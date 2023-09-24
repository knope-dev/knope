# PrepareRelease step

This step:

1. Looks through all commits since the last version tags and parses any [Conventional Commits] it finds.
2. Reads any [Changesets] in the `.changeset` folder (which you can create via [`CreateChangeFile`]). Those files are deleted after being read.
3. Bumps the [semantic version][semantic versioning] of any packages that have changed.
4. Adds a new entry to any affected changelog files.
5. Stages all files modified by this step with Git (effectively, `git add <file>` for versioned files, changelogs, and changesets). This step **does not commit** the changes.

When multiple [packages] are configured—`PrepareRelease` runs for each package independently. The version tag _for that package_ will be the starting point.

```admonish note
The last "version tag" is used as the starting point to read commits—that's the most recent tag that was created by the [`Release`] step. See that step for details on the tagging formats.
```

## Limitations

- The Changelog format is pretty strict. Sections will only be added for [Conventional Commits] and [Changesets] that meet certain requirements. See [Changelog sections](#change-sections) below.
- Knope uses a simpler subset of semantic versioning which you can read about in [`BumpVersion`]
- Knope will not allow you to update the major version of a `go.mod` file in most cases, as the [recommended practice](https://go.dev/blog/v2-go-modules) is to create a new `go.mod` file (in a new directory) for each major version. You can override this behavior using the [`--override-version`] option (to go from `v1` to `v2`) or use [multiple packages](../packages.md#multiple-major-versions-of-go-modules) to support multiple `go.mod` files on different major versions.

## Options

- `allow_empty`: If set to `true`, this step will not fail if there are no changes to release. Defaults to `false`.

## Mono-repos and multiple packages

You can have [multiple packages in one repo](../packages.md#multiple-packages). By default, changesets work with multiple packages and conventional commits apply to _all_ packages. If you want to target specific conventional commits at individual packages, you need use a [conventional commit scope]. This is done by adding a `scopes` array to the [packages] config and adding a [conventional commit scope] to the commits that should not apply to all packages. The following rules apply, in order, with respect to conventional commit scopes:

1. If no packages define `scopes` in their config, all commits apply to all packages. Scopes are not considered by `knope`.
2. If a commit does not have a scope, it applies to all packages.
3. If a commit has a scope, and _any_ package has defined a `scopes` array, the commit will only apply to those packages which have that scope defined in their `scopes` array.

## Changelog format

### Version titles

The title of each version is a combination of its semantic version (e.g., `1.2.3`) and the UTC date of when it was released (e.g., `(2017-04-09)`). UTC is used for simplicity—in practice, the exact _day_ of a release is not usually as important as the general timing.

### Change sections

Sections are only added to the changelog for each version as needed—if there are no commits that meet the requirements for a section, that section will not be added. The built-in sections are:

1. `### Breaking Changes` for anything that triggers a major semantic version increase.
   1. Any commit whose type/scope end in `!` will land in this section **instead** of their default section (if any). So `fix!: a breaking fix` will add the note "a breaking fix" to this section and **nothing** to the "Fixes" section.
   2. If the special `BREAKING CHANGE` footer is used in any commit, the message from that footer (not the main commit message) will be added here. The main commit message will be added as appropriate to the normal section. So a `fix: ` commit with a `BREAKING CHANGE` footer creates entries in both the `### Fixes` section and the `### Breaking Changes` section.
   3. Any changeset with a [change type] of `major` (selecting "Breaking" in [`CreateChangeFile`])
2. `### Features` for any commit with type `feat` (no `!`) or change type `minor` (selecting "Feature" in [`CreateChangeFile`])
3. `### Fixes` for any commit with type `fix` (no `!`) or change type `patch` (selecting "Fix" in [`CreateChangeFile`])
4. `### Notes` for any footer in a conventional commit called `Changelog-Note`. This section name can be changed via [configuration](../packages.md#extra_changelog_sections).
5. Custom sections as defined in the [configuration](../packages.md#extra_changelog_sections).

## Versioning

Versioning is done with the same logic as the [`BumpVersion`] step, but the rule is selected automatically based on the commits since the last version tag and the files present in the `.changeset` directory. Generally, rule selection works as follows:

1. If there are any breaking changes (things in the `### Breaking Changes` section above), the `Major` rule is used.
2. If no breaking changes, but there are any features (things in the `### Features` section above), the `Minor` rule is used.
3. If no breaking changes or features, but there _are_ entries to add to the changelog (fixes, notes, or custom sections) the `Patch` rule is used.
4. If there are no new entries to add to the changelog, version will not be increased, and this step will throw an error (unless the `--dry-run` option is set).

## Examples

### Creating a Pre-release Version

If you include the `prerelease_label` option, the version created will be a pre-release version (treated like `Pre` rule in [`BumpVersion`]). This allows you to collect the commits _so far_ to an impending future version to get them out earlier.

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

If your prerelease workflow is exactly like your release workflow, you can instead temporarily add a prerelease label by passing the [`--prerelease-label` option](../../introduction.md#--prerelease-label) or by setting the `KNOPE_PRERELEASE_LABEL` environment variable. This option overrides any set `prerelease_label` for any workflow run.

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
## 2.0.0-rc.1 (2024-03-14)

### Bug Fixes

- A bug in the first `rc` that we fixed.

## 2.0.0-rc.0 (2024-02-29)

### Breaking Changes

- Cool new API

## 1.14.0 (2023-12-25)

The last 1.x release.
```

Now you're ready to release 2.0.0—the version that's going to come after 2.0.0-rc.1. If you run the defined `release` rule, it will go all the way back to the tag `v1.14.0` and use the commits from that point to create the new version. In the end, you'll get version 2.0.0 with a new changelog entry like this:

```md
## 2.0.0 (2024-04-09)

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
## 1.1.0 (2022-04-09)

### Features

- Add a new --help option to display usage and exit

### Fixes

- Prevent a crash when parsing invalid input
```

And the changelog for `lib` will look like this:

```md
## 0.9.0 (2022-03-14)

### Breaking Changes

- Change the error type of the parse function

### Fixes

- Prevent a crash when parsing invalid input
```

## Errors

The reasons this can fail:

1. The version could not be bumped for some reason.
2. The [packages] section is not configured correctly.
3. There was nothing to release _and_ `allow_empty` was not set to `true`. In this case it exits immediately so that there aren't problems with later steps.

[semantic versioning]: https://semver.org
[`bumpversion`]: ./BumpVersion.md
[packages]: ../packages.md
[`release`]: ./Release.md
[conventional commit scope]: https://www.conventionalcommits.org/en/v1.0.0/#commit-message-with-scope
[conventional commits]: https://www.conventionalcommits.org/en/v1.0.0/
[changesets]: https://github.com/changesets/changesets
[`CreateChangeFile`]: ./CreateChangeFile.md
[change type]: https://github.com/knope-dev/changesets#change-type
