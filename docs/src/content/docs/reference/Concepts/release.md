---
title: Release
---

A release is a snapshot in time of a [package].
You release a version of a package when you want to share it with others, or deploy it somewhere.
In Knope, releasing a package consists of:

1. Determining the new [semantic version]
2. Update all versioned files with the new version
3. Add the details of all changes since the last release to the [changelog]
4. Create a [Git tag](#git-tags)
5. Optionally create a release (as part of the previous step) if [a forge is configured](/reference/concepts/forge)

:::tip

This is how the [built-in release workflow](/reference/default-workflows) works.

:::

## Git tags

When there is a single package, each release gets a Git tag that looks like `v1.2.3`.
When there are multiple packages, each release gets a Git tag that looks like `package_name/v1.2.3`.

:::caution

**A note on Go modules**

Knope does its best to cooperate with Go's requirements for tagging module releases,
however, there are cases where Knope's tagging requirements will conflict with Go's tagging requirements.

For example, if you have a package named `blah` which does _not_ contain the `blah/go.mod` file,
and a package named `something_else` which does contain the `blah/go.mod` file,
then both packages are going to get the `blah/v{Version}` tags,
causing runtime errors during this step.

If you have named packages, it's important that _either_:

1. No package names match the name of a go module
2. All packages with the same name as a go module contain the `go.mod` file for that module

:::

[package]: /reference/concepts/package
[semantic version]: /reference/concepts/semantic-versioning
[changelog]: /reference/concepts/changelog
