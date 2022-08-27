# Release Step

Release the configured [packages]. If there is a [GitHub config] set, this creates a release on GitHub with the same release notes that were added to the changelog (if any). Otherwise, this tags the current commit as a release. In either case, a new Git tag will be created with the package's tag format. The [`PrepareRelease`] step must be run before this one in the same workflow.

## Tagging Format

Whenever this step is run, it will tag the current commit with the new version for each package. If only one package is defined (via the `[package]` section in `knope.toml`), this tag will be v{version} (e.g., v1.0.0 or v1.2.3-rc.4).

If multiple packages are defined, each package gets its own tag in the format {package_name}/v{version} (this is the syntax required for Go modules). See examples below for more illustration.

## Errors

This step will fail if any of the following are true:

1. [`PrepareRelease`] has not run before this step.
2. [GitHub config] is set but Knope cannot communicate with GitHub or the configured token does not have permission to create releases.
3. There is no [GitHub config] set and Knope cannot tag the current commit as a release.

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

# Updates Cargo.lock from Cargo.toml so the versions match.
[[workflow.steps]]
type = "Command"
command = "cargo update -w"

# Add the freshly modified changes
[[workflows.steps]]
type = "Command"
command = "git add Cargo.toml Cargo.lock CHANGELOG.md"

# Commit the changes (make sure this is run on your default branch!)
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

# Updates Cargo.lock from Cargo.toml so the versions match.
[[workflow.steps]]
type = "Command"
command = "cargo update -w"

# Add the freshly modified changes
[[workflows.steps]]
type = "Command"
command = "git add Cargo.toml Cargo.lock CHANGELOG.md"

# Commit the changes (make sure this is your default branch!)
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
[packages.knope]
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

# Updates Cargo.lock from Cargo.toml so the versions match.
[[workflow.steps]]
type = "Command"
command = "cargo update -w"

# Add the freshly modified changes
[[workflows.steps]]
type = "Command"
command = "git add Cargo.lock knope/Cargo.toml knope/CHANGELOG.md knope-utils/Cargo.toml knope-utils/CHANGELOG.md"

# Commit the changes (make sure this is run on your default branch!)
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

[github config]: ../github.md
[`preparerelease`]: PrepareRelease.md
[packages]: ../packages.md
