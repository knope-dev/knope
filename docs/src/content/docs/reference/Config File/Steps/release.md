---
title: Release
---

Release the configured [packages] which need to be released. If there is a [GitHub config] set, this creates a release on GitHub with the same release notes that were added to the changelog (if any). Otherwise, this tags the current commit as a release. In either case, a new Git tag will be created with the package's tag format. You should run [`PrepareRelease`] before this step, though not necessarily in the same workflow. [`PrepareRelease`] will update the package versions without creating a release tag. `Release` will create releases for any packages whose current versions do not match their latest release tag.

## Tagging Format

Whenever this step is run, it will tag the current commit with the new version for each package. If only one package is defined (via the `[package]` section in `knope.toml`), this tag will be v{version} (e.g., v1.0.0 or v1.2.3-rc.4).

If multiple packages are defined, each package gets its own tag in the format {package_name}/v{version} (this is the syntax required for Go modules). See examples below for more illustration.

:::caution
**A note on Go modules**

Knope does its best to place nicely with Go's requirements for tagging module releases, however there are cases where Knope's tagging requirements will conflict with Go's tagging requirements. In particular, if you have a package named `blah` which does _not_ contain the `blah/go.mod` file, and a package named `something_else` which contains the `blah/go.mod` file, then both packages are going to get the `blah/v{Version}` tags, causing runtime errors during this step. If you have named packages, it's important to ensure that _either_:

1. No package names match the name of a go module
2. All packages with the same name as a go module contain the `go.mod` file for that module
   :::

## GitHub Release Notes

There are several different possible release notes formats, depending on how this step is used:

1. If run after a [`PrepareRelease`] step in the same workflow, the release notes will be the same as the changelog section created by [`PrepareRelease`] even if there is no changelog file configured—with the exception that headers are reduced by one level (for example, `####` becomes `###`).
2. If run in a workflow with no [`PrepareRelease`] step before it (the new version was set another way), and there is a changelog file for the package, the release notes will be taken from the relevant changelog section. This section header must match exactly what [`PrepareRelease`] would have created. Headers will be reduced by one level (for example, `####` becomes `###`).
3. If run in a workflow with no [`PrepareRelease`] step before it (the new version was set another way), and there is no changelog file for the package, the release will be created using GitHub's automatic release notes generation.

## GitHub Release Assets

You can optionally include any number of assets which should be uploaded to a freshly-created release via [package assets]. If you do this, the following steps are taken:

1. Create the release in draft mode
2. Upload the assets one at a time
3. Update the release to no longer be a draft (published)

If you have any follow-up workflows triggered by GitHub releases, you can use `on: release: created` to run as soon as the draft is created (without assets) or `on: release: published` to run only after the assets are done uploading.

## Errors

This step will fail if:

1. [GitHub config] is set but Knope cannot create a release on GitHub. For example:
   1. There is no GitHub token set.
   2. The GitHub token does not have permission to create releases.
   3. The release already exists on GitHub (causing a conflict).
2. There is no [GitHub config] set and Knope cannot tag the current commit as a release.
3. Could not find the correct changelog section in the configured changelog file for loading release notes.
4. One of the configured package assets does not exist.

## Examples

### Create a GitHub Release for One Package

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

If `PrepareRelease` set the new version to "1.2.3", then a GitHub release would be created called "1.2.3" with the tag "v1.2.3".

### Git-only Release for One Package

Here's what Knope's config might look like if it were not using GitHub releases:

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

### Create GitHub Releases for Multiple Packages

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

### Create a GitHub Release with Assets

See [Knope's release workflow] and [knope.toml] where we:

1. Prep the release to get the new version and changelog
2. Commit the changes
3. Fan out into several jobs which each check out the changes and build a different binary
4. Create a GitHub release with the new version, changelog, and the binary assets

[github config]: ../github.md
[`preparerelease`]: prepare-release
[packages]: ../packages.md
[package assets]: ../packages.md#assets
[Knope's release workflow]: https://github.com/knope-dev/knope/blob/main/.github/workflows/release.yml
[knope.toml]: https://github.com/knope-dev/knope/blob/main/knope.toml
