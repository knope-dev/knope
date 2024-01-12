---
title: Release
---

Release the configured [packages] which have pending changes.
If there is a [forge config] set,
this creates a release with the same release notes that it added to the changelog (if any).
Otherwise, this tags the current commit as a release.
In either case, this step adds a new Git tag with the package's tag format.
You should run [`PrepareRelease`] before this step, though not necessarily in the same workflow.
[`PrepareRelease`] will update the package versions without creating a release tag.
`Release` will create releases for any packages whose current versions don't match their latest release tag.

## Tagging format

This step tags the current commit with the new version for each package.
If the config file is using the single `[package]` syntax,
or there is no config file, this tag will be v{version} (for example v1.0.0 or v1.2.3-rc.4).

If the config file is using the `[packages.{name}]` syntax,
each package gets its own tag in the format `{name}/v{version}` (this is the syntax required for Go modules).
See examples below for more illustration.

## Release notes

There are several different possible release notes formats:

1. If run after a [`PrepareRelease`] step in the same workflow, the release notes will be the same as the changelog section created by [`PrepareRelease`] even if there is no changelog file configured—with the exception that headers are one level higher (for example, `####` becomes `###`).
2. If run in a workflow with no [`PrepareRelease`] step before it (the new version was set another way), and there is a changelog file for the package, the release notes will be taken from the relevant changelog section. This section header must match exactly what [`PrepareRelease`] would have created. Headers will one level higher (for example, `####` becomes `###`).
3. If run in a workflow with no [`PrepareRelease`] step before it (the new version was set another way), and there is no changelog file for the package, the step will use automatic release notes generation.

## Release assets

You can optionally include any number of assets to include in a release via [package assets].
If you do this, this step will:

1. Create the release in draft mode
2. Upload the assets one at a time
3. Update the release to no longer be a draft (published)

If you have any follow-up workflows triggered by GitHub releases,
you can use `on: release: created` to run as soon as the step creates the draft
(without assets) or `on: release: published` to run only after the assets are uploaded.

## Errors

This step will fail if:

1. [forge config] exists but Knope can't create a release on the forge. For example:
   1. There is no token set.
   2. The token doesn't have permission to create releases.
   3. The release already exists on the forge (causing a conflict).
2. There is no [forge config] set and Knope can't tag the current commit as a release.
3. Could not find the correct changelog section in the configured changelog file for loading release notes.
4. One of the configured package assets doesn't exist.

## Examples

### Create a GitHub release for one package

Here's a simplified version of the release workflow used for Knope.

```toml
[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"

[[workflows]]
name = "release"

# Generates the new changelog and Cargo.toml version based on conventional commits.
[[workflows.steps]]
type = "PrepareRelease"

# Commit the changes that PrepareRelease added
[[workflows.steps]]
type = "Command"
command = "git commit -m \"chore: Bump to version\""
variables = {"version" = "Version"}

# Push the changes to GitHub so the created tag will point to the right place.
[[workflows.steps]]
type = "Command"
command = "git push"

# Create a GitHub release with the new version and release notes created in PrepareRelease. Tag the commit just pushed with the new version.
[[workflows.steps]]
type = "Release"

[github]
owner = "knope-dev"
repo = "knope"
```

If `PrepareRelease` set the new version to "1.2.3" then a GitHub release would be created called "1.2.3" with the tag "v1.2.3".

### Git-only release for one package

Here's what Knope's config might look like if it weren't using GitHub releases:

```toml
[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"

[[workflows]]
name = "release"

# Generates the new changelog and Cargo.toml version based on conventional commits.
[[workflows.steps]]
type = "PrepareRelease"

# Commit the changes that PrepareRelease made
[[workflows.steps]]
type = "Command"
command = "git commit -m \"chore: Bump to version\""
variables = {"version" = "Version"}

# Create a Git tag on the fresh commit (e.g., v1.2.3)
[[workflows.steps]]
type = "Release"

# Push the commit and the new tag to our remote repository.
[[workflows.steps]]
type = "Command"
command = "git push && git push --tags"
```

If `PrepareRelease` set the new version to "1.2.3", then a Git tag would be created called "v1.2.3".

### Create GitHub releases for multiple packages

```toml
[packages.Knope]
versioned_files = ["knope/Cargo.toml"]
changelog = "knope/CHANGELOG.md"

[packages.knope-utils]
versioned_files = ["knope-utils/Cargo.toml"]
changelog = "knope-utils/CHANGELOG.md"

[[workflows]]
name = "release"

# Updates both Cargo.toml files with their respective new versions
[[workflows.steps]]
type = "PrepareRelease"

# Commit the changes that PrepareRelease made
[[workflows.steps]]
type = "Command"
command = "git commit -m \"chore: Prepare releases\""

# Push the changes to GitHub so the created tag will point to the right place.
[[workflows.steps]]
type = "Command"
command = "git push"

# Create a GitHub release for each package.
[[workflows.steps]]
type = "Release"

[github]
owner = "knope-dev"
repo = "knope"
```

If `PrepareRelease` set the new version of the `knope` package to "1.2.3" and `knope-utils` to "0.4.5", then two GitHub release would be created:

1. "knope 1.2.3" with tag "knope/v1.2.3"
2. "knope-utils 0.4.5" with tag "knope-utils/v0.4.5"

### Create a GitHub release with assets

See [Knope's release workflow] and [knope.toml] where we:

1. Prep the release to get the new version and changelog
2. Commit the changes
3. Fan out into several jobs which each check out the changes and build a different binary
4. Create a GitHub release with the new version, changelog, and the binary assets

[forge config]: /reference/concepts/forge
[`preparerelease`]: /reference/config-file/steps/prepare-release
[packages]: /reference/concepts/package
[package assets]: /reference/config-file/packages#assets
[Knope's release workflow]: https://github.com/knope-dev/knope/blob/main/.github/workflows/release.yml
[knope.toml]: https://github.com/knope-dev/knope/blob/main/knope.toml
