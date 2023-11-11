---
title: PrepareRelease
---

This step:

1. Looks through all commits since the last version tags and parses any [Conventional Commits] it finds.
2. Reads any [Changesets] in the `.changeset` folder (which you can create via [`CreateChangeFile`]). Knope deletes those files after reading them.
3. Bumps the [semantic version][semantic versioning] of any packages that have changed.
4. Adds a new entry to any affected [changelog files].
5. Stages all files modified by this step with Git (effectively, `git add <file>` for versioned files, changelogs, and changesets). This step **doesn't commit** the changes.

When multiple [packages] are configured—`PrepareRelease` runs for each package independently. The version tag _for that package_ will be the starting point.

:::note
Knope uses the last "version tag"
as the starting point
to read commits—that's the most recent tag that the [`Release`] step created.
See that step for details on the tagging formats.
:::

## Options

- `allow_empty`: If set to `true`, this step won't fail if there are no changes to release. Defaults to`false`.
- `prerelease_label`: If set, this step will create a [pre-release version] using the specified label. This can also be set dynamically using the [`--prerelease-label` command line argument].
- The [`--override-version` command line argument] can use used to override the version calculated by this step.

## Errors

The reasons this can fail:

1. Knope couldn't bump the version.
2. The [packages section] isn't configured correctly.
3. There was nothing to release _and_ `allow_empty` wasn't set to `true`. In this case it exits immediately so that there aren't problems with later steps.

[semantic versioning]: /reference/concepts/semantic-versioning
[packages]: /reference/concepts/package
[packages section]: /reference/config-file/packages
[`release`]: /reference/config-file/steps/release
[conventional commits]: /reference/concepts/conventional-commits
[changesets]: /reference/concepts/changeset
[`CreateChangeFile`]: /reference/config-file/steps/create-change-file
[pre-release version]: /reference/concepts/semantic-versioning#types-of-releases
[`--prerelease-label` command line argument]: /reference/command-line-arguments#--prerelease-label
[`--override-version` command line argument]: /reference/command-line-arguments#--override-version
[changelog files]: /reference/concepts/changelog
