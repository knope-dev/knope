# Changelog

All notable changes to this project will be documented in this file, versioning follows [Knope's semantic versioning](https://knope.tech/reference/concepts/semantic-versioning).

A breaking change is any change which would produce a different result for the same input.
The inputs are documented [environment variables](https://knope.tech/reference/environment-variables) and [command line arguments](https://knope.tech/reference/command-line-arguments) as well as any files in the current directory.
The results are changes to the current directory, calls to external commands, and interaction with any integrations
(for example the GitHub API).

Notably, anything written to standard output or standard error
(what you see in the terminal) is _not_ considered part of the public API and may change between any versions.

## 0.21.0 (2025-06-23)

### Breaking Changes

#### Change to default handling of top-level `package.json` files

When using the default config (no `[package]` or `[packages]`), Knope will now treat a top-level `package.json` file
which contains a `workspaces` property as the entrypoint into a monorepo and _not_ a single versioned_file package.

#### Support `package-lock.json` files

`package-lock.json` files are [now supported](https://knope.tech/reference/config-file/packages/#package-lockjson)
as `versioned_files` both for single packages and dependencies (in monorepos).

These files will be auto-detected and updated if using the default (no `[package]` or `[packages]`) config, so
this is a breaking change for those users.

### Features

#### Add support for dependencies in `package.json` files

You can now update the `dependencies` and `devDependencies` of a `package.json` like this:

```toml
[package]
versioned_files = [
  # Update @my/package-name in dependencies and devDependencies whenever this package version updates
  { path = "package.json", dependency = "@my/package-name" }
]
```

#### Support for NPM workspaces

Knope now supports NPM-style workspaces [when using the default config](https://knope.tech/reference/default-config/#npm-workspaces)!

### Fixes

#### Fix multiple versioned files with same path

Previously, if you referenced the same file multiple times in versioned_files, only the first instance would apply.
Now, if you reference the same path multiple times, each instance will be processed sequentially.

Consider a mono-repo where every package should be versioned in lockstep:

```toml
[package]
versioned_files = [
  "package.json",
  { path = "package.json", dependency = "aDependency" },  # Before this fix, this was ignored
  "aDependency/package.json"
]
```

This use-case is now supported!

## 0.20.0 (2025-05-03)

### Breaking Changes

#### Add default variables

[Default variables](https://knope.tech/reference/config-file/variables/#defaults) will now apply anywhere they can be
used—including the `Command` and `CreatePullRequest` steps.

If any of the defaults, like `$version` or `$changelog`, appear in a variable-supporting location
and you don't have explicit `variables =` set for that step, this is a breaking change.

### Fixes

#### Fix detecting previous releases with no changelog

Previously, if you ran a `PrepareRelease` step with no `changelog` to modify in one workflow and then a `Release` step
in a separate workflow, `Release` would fail to create a Git tag.

Now, a release with "no notes" will properly be created if the last Git tag doesn't match the current version of
a file.
If a forge is configured, a release will be created on that forge without any notes,
and a title simply containing the version.

Fixes #1267

#### `PrepareRelease` will allow empty releases when using `--override-version`

Fixes #1423

## 0.19.2 (2025-04-05)

### Features

- Print each step before it runs when `--verbose` is set (#1399)

## 0.19.1 (2025-04-05)

### Features

#### Add support for Tauri config files

Supports [Tauri configuration files](https://v1.tauri.app/v1/references/configuration-files) in json format named `tauri.conf.json`, `tauri.macos.conf.json`, `tauri.windows.conf.json`, and `tauri.linux.conf.json`.

## 0.19.0 (2025-03-23)

### Breaking Changes

#### Updated config locations

Knope is internally using a newer crate for storing persistent config, so some things (like GitHub tokens) may need to
be updated on next run.

### Documentation

#### Document creating releases with Knope Bot

[Knope Bot can make creating releases easier than ever now!](https://knope.tech/tutorials/bot-workflow/)

## 0.18.5 (2025-03-12)

### Features

#### Add versioning support for `gleam.toml` files

##1352 by @dbanty

Closes #1350

## 0.18.4 (2025-03-08)

### Features

#### Add basic support for versioning of `pom.xml` files

`pom.xml` must contain a `<version>` field in the `<project>` section.
Dependencies and multi-module projects are not yet supported.

### Fixes

#### Don't normalize existing changeset file names

When a new change file is created via `CreateChangeFile`,
its name is generated by converting the summary of the change to all lowercase letters and underscores.
This same normalization was happening when opening an _existing_ change file, which prevented that change file from
being deleted during a `PrepareRelease` step if it didn't match the normalized version
(for example, was created with a different tool or by hand).
This has been fixed.

Fixes #1340

### Notes

- Update to Rust edition 2024 and MSRV 1.85

## 0.18.3 (2025-01-21)

### Fixes

- Improve error messages when GitHub releases fail

## 0.18.2 (2025-01-01)

### Fixes

- Fix panic when Cargo.toml is missing a `version`
- Remove warning for Cargo.lock version 4

## 0.18.1 (2024-09-15)

### Features

#### Support glob patterns for package assets

You can now provide a glob pattern when defining package assets instead of specifying each file individually in a list.
This is especially useful when your asset names are dynamic (containing a version, date, or hash, for example) or
when different releases have different assets.

Knope will _not_ error if the glob pattern doesn't match any files.
You can't combine glob patterns with individual file names.

```toml
[package]
assets = "assets/*"
```

## 0.18.0 (2024-08-18)

### Breaking Changes

#### Auto-update Cargo workspace dependencies when using default config

If using the Cargo workspace [default configuration](https://knope.tech/reference/default-config/#cargo-workspaces),
Knope will now attempt to automatically update the version of workspace members in dependencies _and_ the workspace `Cargo.lock`.

To avoid this, use `knope --generate` to create a manual config file and customize the behavior.

#### Don't create _any_ go module tags that match package names

Knope already avoided creating duplicate tags for Go modules which match tags that would be created by the `Release` step for the package.
Now, Knope won't create a Go module tag if it matches a release tag for _any_ configured package, to avoid potential conflicts.

### Features

#### Support for `Cargo.lock` in `versioned_files`

Dependencies within a `Cargo.lock` [can now be updated](https://knope.tech/reference/config-file/packages#cargolock).

#### Support for dependencies within `Cargo.toml`

Dependencies within a `Cargo.toml` file [can now be updated](https://knope.tech/reference/config-file/packages/)
as part of `versioned_files`.

### Fixes

#### Deduplicate release actions

Knope now collects all actions to be performed across all packages and runs them at once with deduplication.

This means that if multiple packages write to the same `versioned_file`, for example, the file will only be written
a single time.
Changesets will also only be deleted once, files will be staged to Git only once, etc.

This mostly only impacts the output during `--dry-run` or `--verbose`, but is especially important for the new
dependency updating and lockfile support.

## 0.17.0 (2024-08-04)

### Breaking Changes

#### Forge date now matches CHANGELOG date

If you prepare a release and generate a changelog Markdown file in one workflow, then create a forge release in a
separate workflow, the forge release date will now match the changelog date (if any). Previously, the forge release got
the current date (at the time of running the workflow).

#### Match scope-filtering behavior to docs

The docs state, in regard to a `package.scopes` config, "if not defined, Knope will consider all scopes."

This is the intended behavior, but wasn't true until now. The actual behavior, for multi-package repos, was that if
_any_ package had scopes defined, _all_ would start filtering scopes.

This has been corrected, packages are now more independent in their scope filtering behavior.

#### Properly use case insensitivity when checking conventional commits

Per the [conventional commits spec](https://www.conventionalcommits.org/en/v1.0.0/#specification) all units of a
conventional commit are case-insensitive.
Until now, Knope was treating commit footers and scopes as case-sensitive. This has been corrected, which may result
in different behavior for some projects.

## 0.16.2 (2024-04-14)

### Features

#### Add `get-version` default workflow

For **single-package repositories** with no custom workflows defined,
there is now a [default workflow](https://knope.tech/reference/default-config/#workflows) called `get-version` that
prints out the current package version.

If you want similar functionality for multi-package repositories, please add your ideas to [issue #988](https://github.com/knope-dev/knope/issues/988).

Thanks to @BatmanAoD for the suggestion and @alex-way for the implementation!

PR #994 closed #885.

#### Add option to ignore conventional commits

You can now add `ignore_conventional_commits = true` to a [`PrepareRelease` step](https://knope.tech/reference/config-file/steps/prepare-release/)
to ignore commit messages (and only consider changesets):

```toml
[[workflows.steps]]
type = "PrepareRelease"
ignore_conventional_commits = true
```

PR #1008 closes #924. Thanks for the suggestion @ematipico!

### Fixes

- Allow omitting the `variables` field for `CreatePullRequest` title and body

### Documentation

#### Created a new recipe for converting a single-package repo into a monorepo

Knope itself is now a monorepo—the process of converting it was documented [here](https:/knope.tech/recipes/convert-to-monorepo).

## 0.16.1 (2024-03-24)

### Features

#### Add `help_text` option to workflows

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

#### Use bullets to describe simple changes

The previous changelog & forge release format used headers for the summary of all changes, these entries were hard
to follow for simple changes like this:

```markdown
### Features

#### A feature

#### Another header with no content in between?
```

Now, _simple_ changes are described with bullets at the _top_ of the section. More complex changes will come after
any bullets, using the previous format:

```markdown
### Features

- A simple feature
- Another simple feature

#### A complex feature

Some details about that feature
```

Right now, a simple change is any change which comes from a conventional commit (whether from the commit summary or
from a footer) _or_ a changeset with only a header in it. Here are three simple changes:

```
feat: A simple feature

Changelog-Note: A note entry
```

```markdown
---
default: minor
---

# A simple feature with no description
```

A complex change is any changeset which has content (not just empty lines) below the header.

PR #969 implemented #930. Thanks for the suggestion @ematipico!

## 0.16.0 (2024-03-20)

### Breaking Changes

#### Don't delete changesets for prereleases

Previously, using `PrepareRelease` to create a prerelease (for example, with `--prerelease-label`) would delete all
changesets, just like a full release. This was a bug, but the fix is a breaking change if you were
relying on that behavior.

### Features

#### Add a `shell` variable for `Command` steps

You can now add `shell=true` to a `Command` step to run the command in the current shell.
This lets you opt in to the pre-0.15.0 behavior.

```toml
[[workflows.steps]]
type = "Command"
command = "echo $AN_ENV_VAR"
shell = true
```

## 0.15.0 (2024-03-18)

### Breaking Changes

#### Don't run `Command` steps in shell

The `Command` step no longer attempts to run the command in a default shell for the detected operating system.
This fixes a compatibility issue with Windows.

If this change doesn't work for your workflow, please open an issue describing your need so we can fix it.

Notably, using `&&` in a command (as was the case for some default workflows) will no longer work. Instead, split this
into multiple `Command` steps.

PR #919 closes issue #918. Thanks for reporting @alex-way!

## 0.14.1 (2024-02-23)

### Features

#### Add `ignore_go_major_versioning` option

You can now set `ignore_go_major_versioning = true` for a package in
`knope.toml` to turn off the major version validation & updating in `go.mod` files.

More details in [the new docs](https://knope.tech/reference/config-file/packages/#ignore_go_major_versioning).

Closes #863, thanks for the suggestion @BatmanAoD!

## 0.14.0 (2024-02-04)

### Breaking Changes

#### `Cargo.toml` files must now have a `package.name` property

This was already required by Cargo, but wasn't enforced by Knope until now. Before, a `Cargo.toml` file like

```toml
[package]
version = "0.1.0"
```

was acceptable, but now it must be

```toml
[package]
name = "my-package"
version = "0.1.0"
```

### Features

#### Add basic Cargo workspace support

If you have a `Cargo.toml` file in the working directory which represents a Cargo workspace containing fixed members, like:

```toml
[workspace]
members = [
  "my-package",
  "my-other-package",
]
```

then Knope will now treat each member like a package.
There must be a `Cargo.toml` file in each member directory, or Knope will error.

This doesn't work with path globbing yet, only manual directory entries. See [the new docs](https://knope.tech/reference/default-config/#cargo-workspaces) for more details.

#### Use default packages/workflows even when `knope.toml` exists

If you define a `knope.toml` file without any packages, Knope will assume the default packages (as if you had no `knope.toml` file at all).

Likewise, if you have no `[[workflows]]` in a `knope.toml` file, Knope will assume the default workflows.

### Fixes

#### Homebrew tap now contains Apple Silicon binaries and auto-publishes new versions (#827)

## 0.13.4 (2024-01-13)

### Features

#### Gitea support

PR #759 closed issue #743. Thank you, @FallenValkyrie!

- Added Support for Gitea in the `CreatePullRequest` step
- Added Support for Gitea in the `Release` step
- Added A new `SelectGiteaIssue` step
- Add support to generate Gitea config from known public Gitea instances

To use these new steps, just add a new section to your configuration, like this:

```toml
[gitea]
repo = "knope"
owner = "knope-dev"
host = "https://codeberg.org"
```

You can now use the supported steps in the same way as their GitHub equivalents.

> [!TIP]
> Knope can now generate a configuration for you, if your repository's remote is one of the known
> public Gitea instances. Currently only [Codeberg](https://codeberg.org) is supported,
> but feel free to add more [here](https://github.com/knope-dev/knope/blob/main/src/config/toml/config.rs#L90).

## 0.13.3 (2023-12-17)

### Features

#### Support `pubspec.yaml` in `versioned_files`

Knope can now version Dart projects! You can now add a `pubspec.yaml` file to your `package.versioned_files`.

PR #732 closes #731. Thanks @FallenValkyrie!

## 0.13.2 (2023-11-11)

### Fixes

#### Add `[github]` section with `knope --generate` (#666)

#### `--dry-run` will no longer report deleting the same changeset twice (#668)

#### `CreateChangeFile` (like `knope document-change`) now prints package names. (#669)

## 0.13.1 (2023-11-05)

### Documentation

#### There is a new docs website

Check out <https://knope.tech/> to see the new docs, and please report any errors or gaps!
All error messages within Knope should be updated to point to the new docs.
If any are still pointed at GitHub pages (as of this version), that's a bug!

As part of this, you can also now install Knope through [Chocolatey and Homebrew](https://knope.tech/installation)!

## 0.13.0 (2023-10-22)

### Breaking Changes

#### Change changeset title level

The level of the title of a changeset no longer impacts the level of the release header in the changelog. To make this more obvious, changeset title are now level one headers by default. This is a breaking change because older versions of Knope will no longer properly handle the changesets from newer versions of Knope.

#### Change where new versions are inserted in changelog

In practice, this will not impact most changelogs, however, previous versions of Knope looked for the first header at a certain level (e.g., starting with `## `) and inserted the new version right before that. Now, Knope looks for the first header that is parseable as a semver version (e.g., `## 1.2.3`) and inserts the new version right before that.

This _will_ make it harder to adopt Knope in projects that have an existing changelog which is not of the same format,
but it makes inserting the new version in the changelog more robust.

### Features

#### Allow overriding built-in changelog sections

If you don't want to use the default changelog sections of "Breaking changes", "Features",
and "Fixes", you can now override them by using the equivalent changeset types!
Overriding them resets their position in the changelog, so you probably want to reset _all_ of them if you reset any.
This looks like:

```toml
[package]
extra_changelog_sections = [
    { type = "major", name = "❗️Breaking ❗" },
    { type = "minor", name = "🚀 Features" },
    { type = "patch", name = "🐛 Fixes" },
    { footer = "Changelog-Note", name = "📝 Notes" },
]
```

#### Support level-one release headers in changelogs

If the last release in a changelog file has a level-one header instead of Knope's default of level-two, new releases will be created with level-one headers as well. Sections will then be level two instead of level three.

## 0.12.0 (2023-09-30)

### Breaking Changes

#### Prevent bumping major version of a `go.mod` file

According to [the docs](https://go.dev/blog/v2-go-modules), aside from the `v0` -> `v1` transition, `go.mod` files should not be updated for new major versions, but instead a new `v{major}` directory should be created with a new `go.mod` file. This is for compatibility with older versions of Go tools.

In order to prevent someone from accidentally doing the wrong thing, Knope will no longer bump a `go.mod` file to `v2` unless `--override-version` is used to bypass this check. Additionally, if a `go.mod` file is in the matching versioned directory (e.g., the `go.mod` file ending in `/v2` is under a directory called `v2`), Knope will not allow the major version of _that_ file to be bumped, as it would break the package.

### Fixes

#### Handle version-specific go modules correctly

Fixes #584 from @BatmanAoD.

If you have a `go.mod` file representing a specific major version in a directory (as recommended in [the go docs](https://go.dev/blog/v2-go-modules)), Knope will now tag it correctly. Previously, a `v2/go.mod` file would generate a tag like `v2/v2.1.3`. Now, it will generate a tag like `v2.1.3`.

Additionally, when determining the _current_ version for a `go.mod` file, only tags which match the major version of the `go.mod` file will be considered.

#### Properly version named packages containing a root `go.mod` file

Consider this package config in a `knope.toml`:

```toml
[packages.something]
versioned_files = ["go.mod"]
```

The `Release` step previously (and will still) add a tag like `something/v1.2.3`, however the correct Go module tag is `v1.2.3` (without the package name prefix). Knope will now correctly add this second tag (previously, top-level tags were only added for single-package repos).

### Documentation

#### Document conflict between package names and go module names

It is possible to write a `knope.toml` file which will cause conflicting tags during the `Release` step if you have `go.mod` files in nested directories. [This is now documented](https://knope.tech/reference/config-file/steps/release/).

## 0.11.1 (2023-09-22)

### Features

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

[the `Command` step]: https://knope.tech/reference/config-file/steps/command/

#### Added an `allow_empty` option to the `PrepareRelease` step

Closes #416

If you want to run `PrepareRelease` on every push to a branch without it failing when there's nothing to release, you can now include the `allow_empty` option like this:

```toml
[[workflows.steps]]
type = "PrepareRelease"
allow_empty = true
```

Then, you can use some logic to gracefully skip the rest of your CI process if there is nothing to release. For example, in GitHub Actions, you could do something like this:

```yaml
- name: Prepare Release
  run: knope prepare-release
- name: Check for Release
  id: status
  run: echo ready=$(if [[ `git status --porcelain` ]]; then echo "true"; else echo "false"; fi;) >> $GITHUB_OUTPUT
- name: Release
  if: steps.status.outputs.ready == 'true'
  run: knope release
```

This allows you to differentiate between there being nothing to release and the `PrepareRelease` step failing for other reasons.

#### New `CreatePullRequest` step

The new [`CreatePullRequest` step] allows you to create or update a pull request on GitHub. It's designed to be a nice way to preview and accept new releases via a pull request workflow, but could certainly work for more contexts as well! To see an example of the new PR-based release workflow, check out [Knope's prepare-release workflow] and [Knope's release workflow].

[`CreatePullRequest` step]: https://knope.tech/reference/config-file/steps/create-pull-request/
[Knope's prepare-release workflow]: https://github.com/knope-dev/knope/blob/e7292fa746fe1d81b84e5848815c02a0d8fc6f95/.github/workflows/prepare_release.yml
[knope's release workflow]: https://github.com/knope-dev/knope/blob/e7292fa746fe1d81b84e5848815c02a0d8fc6f95/.github/workflows/release.yml

### Fixes

#### Only consider prereleases newer than the last stable

This fixes a regression in the previous version of Knope where _all_ prereleases would be considered, rather than just those tagged after the latest stable version.

### Documentation

#### GitHub Actions Recipes

There's a [new section of the docs](https://knope.tech/recipes/1-preview-releases-with-pull-requests/) with some recipes for using Knope in GitHub Actions. If you have suggestions for additional recipes, please open a [discussion](https://github.com/knope-dev/knope/discussions)!

## 0.11.0 (2023-09-13)

### Breaking Changes

#### Ignore unreachable tags when determining version

PR #574 fixes issue #505 from @BatmanAoD.

Previously, the latests tags were always used to determine the current version, **even if those tags were not reachable from `HEAD`**. Now, only reachable tags will be considered. Use the `--verbose` flag to see tags which are being ignored.

### Fixes

#### Consistent commit selection in branching histories

PR #574 fixes issue #505 from @BatmanAoD.

Previous versions of Knope did not handle branching histories correctly. In some cases, this could result in commits from previous stable releases being included in a new release. It could _also_ result in missing some commits that _should_ have been included. This has been fixed—Knope should provide you the same commit list that `git rev-list {previous_stable_tag}..HEAD` would.

## 0.10.0 (2023-09-09)

### Breaking Changes

#### Reworked Go versioning

In order to support running `Release` in a separate workflow from `PrepareRelease` and to fix a bug relating to Go module tags (when in a subdirectory), Knope will now store the full package version in a comment in the `go.mod` file and use that version as the source of truth for the package version. This has a couple of implications:

1. If you already have a comment on the `module` line in `go.mod` which matches the correct format, Knope may not be able to determine the version correctly.
2. If you have a comment on that line which does _not_ match the format, it will be erased the next time Knope bumps the version.

In either case, the solution is to erase or move that comment. Here is the syntax that Knope is looking for:

`module {ModulePath} // v{Version}`

If that comment does not exist, Knope will revert to looking for the latest relevant Git tag instead to _determine_ the version, but will still write the comment to the `go.mod` file when bumping the version.

### Features

#### `--verbose` flag

PR #545 closed issue #534 by @BatmanAoD.

There is now a global `--verbose` flag that will spit out _lots_ of extra info to stdout to assist with debugging. Right now, only the process for determining and bumping new package versions is instrumented, open issues if you need more info!

#### Allow `Release` step to be in separate workflow than `PrepareRelease`

Previously, you needed to have a `PrepareRelease` step earlier in the same workflow if you wanted to use the `Release` step. Now, if you run a `Release` step _without_ a `PrepareRelease` step, Knope will check Git tags and versioned files to figure out if there's something new to release. This is especially useful if you want to build release assets using the new version (determined by `PrepareRelease`) before actually releasing them (using `Release`).

#### Upload assets to GitHub Releases

You can now add assets to a package like this:

```toml
[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"

[[package.assets]]
path = "artifact/knope-x86_64-unknown-linux-musl.tgz"
name = "knope-x86_64-unknown-linux-musl.tgz"  # Optional, defaults to file name (so this `name` is doing nothing)

[[package.assets]]
path = "artifact/knope-x86_64-pc-windows-msvc.tgz"
```

When running the `Release` step with a valid `[github]` config, instead of immediately creating the release, Knope will:

1. Create a draft release
2. Upload all listed assets (erroring if any don't exist)
3. Publish the release

### Fixes

#### Use the correct tags for `go.mod` files in subdirectories

PR #544 fixed issue #502 by @BatmanAoD.

Previously, the version for a `go.mod` file was determined by the package tag, named `v{Version}` for single packages or `{PackageName}/v{Version}` for named packages. This worked when the `go.mod` file was in the root of the repository or a directory named `{PackageName}` (respectively), but not when it was in a different directory. Now, the version tag, both for determining the current version and creating a new release, will correctly be determined by the name of the directory the `go.mod` file is in (relative to the working directory). The existing package tagging strategy remains unchanged.

For example, consider this `knope.toml` file:

```toml
[package]
versioned_files = ["some_dir/go.mod"]
```

Previous to this release, creating the version `1.2.3` would only create a tag `v1.2.3`. Now, it will _additionally_ create the tag `some_dir/v1.2.3`.

## 0.9.0 (2023-08-10)

### Breaking Changes

#### Removed the deprecated `[[packages]]` syntax

If you're using the old syntax, run `knope --upgrade` _before_ switching to this version.

#### `--generate` can no longer be used if a `knope.toml` file already exists

#### Workflows can no longer be selected interactively

Previously, it was valid to invoke `knope` with no arguments, and the user would be prompted interactively to select a workflow. Now, a workflow must be provided as a positional argument, for example, `knope release`.

#### The `--prerelease-label` option can only be provided after a workflow

Previously, the `--prerelease-label` CLI option was always available globally and would simply be ignored if it was not useful for the selected workflow. Now, it can only be provided _after_ the name of a workflow which can use the option (right now, only a workflow which contains a [`PrepareRelease`](https://knope.tech/reference/config-file/steps/prepare-release/) step). For example, with the default workflow, `knope release --prerelease-label="rc"` is valid, but **none of these are valid**:

- `knope --prerelease-label="rc" release`
- `knope document-change --prerelease-label="rc"`

#### `--upgrade` can no longer be used if there is no `knope.toml` file

#### `--validate` can no longer be used if there is no `knope.toml` file

### Features

#### Added the `--override-version` option to manually set the next version

Allows you to manually determine the next version for a [`BumpVersion`] or [`PrepareRelease`] instead of using a semantic versioning rule. This option can only be provided after a workflow which contains a relevant step. This has two formats, depending on whether there is one package or multiple packages:

1. `--override-version 1.0.0` will set the version to `1.0.0` if there is only one package configured (error if multiple packages are configured).
2. `--override-version first-package=1.0.0 --override-version second-package=2.0.0` will set the version of `first-package` to `1.0.0` and `second-package` to `2.0.0` if there are multiple packages configured (error if only one package is configured).

This closes [#497](https://github.com/knope-dev/knope/issues/497).

#### `knope --help` now lists all available workflows

## 0.8.0 (2023-06-19)

### Breaking Changes

#### Changelog entries now use 4th level headers instead of bullets

In order to support more detailed changelogs via [changesets](https://knope.tech/reference/concepts/changeset/) (like the extra text you're seeing right now!) instead of each change entry being a single bullet under the appropriate category (e.g., `### Breaking Changes` above), it will be a fourth-level header (`####`). So, where _this_ changelog entry would have currently looked like this:

```markdown
### Breaking Changes

- Changelog entries now use 4th level headers instead of bullets
```

It now looks like what you're seeing:

```markdown
### Breaking Changes

#### Changelog entries now use 4th level headers instead of bullets

... recursion omitted
```

If a change note starts with `#### ` already (like in changesets), it will be left alone.

### Features

#### Move GitHub Release headers up a level (#467, #472)

#### Added dates to version titles

There are now release dates in both changelogs and version names on GitHub. This probably won't break your releases, but you will have a different format for release notes which could be jarring. The date is in the format `YYYY-MM-DD` and will always be based on UTC time (so if you do a release late at night on the east coast of the United States, the date will be the next day).

Previously, the changelog entry title would look like this:

```markdown
## 1.0.0
```

And now it will look like this:

```markdown
## 1.0.0 (2023-06-10)
```

#### Report files to be added to git in `--dry-run`

The [`PrepareRelease`](https://knope.tech/reference/config-file/steps/prepare-release/) step adds modified files to Git. Now, when running with the `--dry-run` option, it will report which files would be added to Git (for easier debugging).

> Note: The default `knope release` workflow includes this [`PrepareRelease`] step.

#### Remove duplicate version from GitHub release name

Release notes in GitHub releases used to copy the entire section of the changelog, including the version number. Because the name of the release also includes the version, you'd see the version twice, like:

```markdown
# 1.0.0

## 1.0.0

... notes here
```

Now, that second `## 1.0.0` is omitted from the body of the release.

#### Added support for changesets

Leveraging the new [changesets crate](https://github.com/knope-dev/changesets), Knope now supports [changesets](https://github.com/changesets/changesets)! In short, you can run `knope document-change` (if using default workflows) or add the new [`CreateChangeFile`] step to a workflow to generate a new Markdown file in the `.changeset` directory. You can then fill in any additional details below the generated header in the generated Markdown file. The next time the `PrepareRelease` step runs (e.g., in the default `knope release` workflow), all change files will be consumed to help generate a new version and changelog (along with any conventional commits).

For additional details, see:

- [`PrepareRelease` step](https://knope.tech/reference/config-file/steps/prepare-release/)
- [`CreateChangeFile` step](https://knope.tech/reference/config-file/steps/create-change-file/)
- [Customizing changelogs](https://knope.tech/recipes/customizing-changelogs/)

## 0.7.4

### Features

- Allow more changelog sections via `extra_changelog_sections` config or the default `Changelog-Note` commit footer. (#450)

## 0.7.3

### Features

- Remove any potential panics (#429)

### Fixes

- Handle merge commits in history. Thanks @erichulburd! (#443)

## 0.7.2

### Fixes

- Avoid GLIBC issues by skipping gnu builds (#407)

## 0.7.1

### Fixes

- Fix building from source / `cargo install` by upgrading to `gix`. (#383)

## 0.7.0

### Breaking Changes

- Handling of pre-release versions has been reworked to handle many more edge cases, which results in more correct (but different) behavior.

### Fixes

- Check all relevant pre-release versions, not just the latest [#334, #346]. Thanks @Shadow53!

## 0.6.3

### Fixes

- determining latest versions from tags (#323)

## 0.6.2

### Features

- Allow running default workflows (those that would be created by`--generate`) with no `knope.toml` file. (#286)

## 0.6.1

### Features

- Support PEP621 in `pyproject.toml`. (#274)

## 0.6.0

### Breaking Changes

- `PrepareRelease` now `git add`s all modified files. (#267)

### Fixes

- Remote parsing in `--generate` (#268)

## 0.5.1

### Fixes

- Do not error on `--validate` or `--dry-run` when no release will be created for `PrepareRelease`. (#265)

## 0.5.0

### Breaking Changes

- `PrepareRelease` will now error if no version-altering changes are detected.

### Features

- Support limiting commits to scopes per package. (#262)
- Allow multiple defined packages, deprecate old `[[packages]]` syntax. (#257)

## 0.4.3

### Features

- `prerelease_label` can be set at runtime with `--prerelease-label` or `KNOPE_PRERELEASE_LABEL`. (#247)

## 0.4.2

### Features

- `PrepareRelease` will create a `changelog` file if it was missing. (#240)

### Fixes

- Always set a committer on tags to resolve compatibility issue with GitLab. (#236)
- Include file paths in file-related errors. (#239)

## 0.4.1

### Features

- Add support for `go.mod` in `versioned_files`. (#228)

### Fixes

- Tags were being processed backwards starting in 0.4.0 (#227)

## 0.4.0

### Breaking Changes

- Always read all commits from previous stable release tag—not from most recent tag. This tag must be in the format v<semantic_version> (e.g., v1.2.3). If your last tag does not match that format, add a new tag before running the new version of Knope.
- When creating GitHub releases, prefix the tag with `v` (e.g., `v1.2.3) as is the custom for most tools.

### Features

- Support reading commits from projects with no tags yet. (#225)
- Support pulling current version from tags. (#224)
- Allow the `Release` step to run without GitHub config—creating a tag on release. (#216)
- Support installs from cargo-binstall

### Fixes

- update rust crate git-conventional to 0.12.0 (#203)

## 0.4.0-rc.4

## 0.4.0-rc.3

## 0.4.0-rc.2

## 0.4.0-rc.1

### Breaking Changes

- When creating GitHub releases, prefix the tag with `v` (e.g., `v1.2.3) as is the custom for most tools.

### Features

- Support installs from cargo-binstall

### Fixes

- update rust crate git-conventional to 0.12.0 (#203)

## 0.4.0-rc.0

### Breaking Changes

- When creating GitHub releases, prefix the tag with `v` (e.g., `v1.2.3) as is the custom for most tools.

### Features

- Support installs from cargo-binstall

### Fixes

- update rust crate git-conventional to 0.12.0 (#203)

## 0.3.0

### Breaking Changes

- `BumpVersion` and `PrepareRelease` now require setting a `[[packages]]` field in `knope.toml`. The path to a changelog file is no longer defined with `changelog_path` in the `PrepareRelease` step. Instead, it is set as `changelog` in `[[packages]]`.

### Features

- Support multiple versioned_files in one package.
- Specify which versioned file to bump instead of picking automatically. (#182)
- Support loading GitHub credentials from `GITHUB_TOKEN` env var (#172)

### Fixes

- update rust crate thiserror to 1.0.31 (#171)

## 0.2.1-rc.0

### Features

- Support loading GitHub credentials from `GITHUB_TOKEN` env var (#172)

### Fixes

- update rust crate thiserror to 1.0.31 (#171)

## 0.2.0

### Breaking Changes

- Rename to Knope, which has much more positive associations. (#161)
- Allow switching between pre-release prefixes instead of erroring (e.g. -alpha.1 -> -beta.0)
- `BumpVersion` now takes a `label` parameter for the `Pre` rule instead of `value`.
- `UpdateProjectFromCommits` step has been renamed to `PrepareRelease`.

### Features

- Add a `--generate` option for generating a brand-new config file with a default `release` workflow. (#159)
- Add top-level `--validate` and per-workflow `--dry-run` options. (#158)
- Add a `dry-run` option to the `PrepareRelease` step. (#139, #137)
- Add a `Release` step for generating GitHub releases. (#136)
- Support pre-releases in `UpdateProjectFromCommits`. (#132)

### Fixes

- update rust crate git2 to 0.14.2 (#157)
- Bump version before adding a pre-release component.
- Stop parsing Markdown in Changelogs to avoid errors in unimplemented features. (#127)

## 0.1.5

### Fixes

- Properly handle Windows newlines in commits (#119)

## 0.1.4

### Features

- Support the BREAKING CHANGE footer with a separate breaking description.

### Fixes

- update rust crate dialoguer to 0.9.0 and console to 0.15.0 (#114)

## 0.1.3

### Features

- You can now pass the name of a workflow as an argument to bypass the selection prompt (closes #24)

### Fixes

- Commits with extra whitespace at the end were not being recorded properly

## 0.1.2

### Fixes

- Retain property order when writing changes to package.json (#75)

## 0.1.1

### Features

- Specify a path to a changelog file in UpdateProjectFromCommits (closes #27) (#71)
- Use special version bumping rules for versions that start with 0.x (closes #37) (#65)

## 0.1.0 - 2021-01-29

- Initial release
