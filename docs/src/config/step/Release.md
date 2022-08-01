# Release Step

Release the configured [package]. If there is a [GitHub config] set, this creates a release on GitHub with the same release notes that were added to the changelog (if any). Otherwise, this tags the current commit as a release. In either case, a new Git tag will be created with the format `v{version}`. The [PrepareRelease] step must be run before this one in the same workflow.

## Errors

This step will fail if any of the following are true:

1. [PrepareRelease] has not run before this step.
2. [GitHub config] is set but Knope cannot communicate with GitHub or the configured token does not have permission to create releases.
3. There is no [GitHub config] set and Knope cannot tag the current commit as a release.

## Examples

### Create a GitHub Release

Here's a simplified version of the release workflow used for Knope.

```toml
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

# Push the changes to GitHub so the created tag will point to the right place.
[[workflows.steps]]
type = "Command"
command = "git push"

# Create a GitHub release with the new version and release notes created in PrepareRelease. Tag the commit just pushed
# with the new version.
[[workflows.steps]]
type = "Release"

[github]
owner = "knope-dev"
repo = "knope"
```

### Tag the Current Commit as a Release

Here's what Knope's config might look like it it were not using GitHub releases:

```toml
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

[github config]: ../github.md
[preparerelease]: PrepareRelease.md
[package]: ../packages.md
