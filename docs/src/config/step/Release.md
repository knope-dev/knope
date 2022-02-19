# Release Step

Create a new GitHub release with the new version and release notes created in [PrepareRelease]

## Errors

This step will fail if any of the following are true:

1. [PrepareRelease] has not run before this step.
2. Dobby cannot communicate with GitHub.
3. There is no [GitHub config] set.
4. User does not select an issue.

## Example

Here's a simplified version of the release workflow used for Dobby.

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
```

[issueselected]: ../../state/IssueSelected.md
[github config]: ../github.md
[preparerelease]: PrepareRelease.md
