---
title: Semantic versioning
---

Knope changes the version of a package using a subset of [semantic versioning](https://semver.org).
There are three types of changes, and two types of releases.

## Types of changes

Every semantic version has three main parts separated by a period: `<major>.<minor>.<patch>`.
For example, in the version `1.2.3`, the major version is `1`, the minor version is `2`, and the patch version is `3`.
When you make a change to a package, you need to decide which part of the version number it should change, which we call the change type.

### Major changes

A major change (also called a breaking change) is the most important type of change.
Users of your package **should not upgrade** until they've read any of these changes.
Typically, this is because the change is backwards-incompatible.
If there are any major changes in a release,
the major version is incremented and the minor and patch versions are reset to zero _regardless of any other changes_.
For example, `1.2.3` becomes `2.0.0`.

### Minor changes

A minor change is something new about the package that users may be interested in,
which requires an action to take advantage of.
If users don't read these changes, nothing bad will happen,
but if they _do_, they may be able to make their experience better.
This is usually a new feature.

If there are any minor changes in a release, and no major changes,
the minor version is incremented and the patch version is reset to zero _regardless of any patch changes_. For example, `1.2.3` becomes `1.3.0`.

### Patch changes

A patch change is an improvement
that users probably don't need to read about unless they had a particular problem which might now be solved.
For example, a bug was fixed that some users maybe were working around and no longer need to.

If there are any patch changes in a release, and no major or minor changes, the patch version is incremented. For example, `1.2.3` becomes `1.2.4`.

## Types of releases

There are two types of releases: pre-releases and final releases (also just called releases).
A final release is one with the sorts of versions we've been looking at so far, the three semantic components. Most releases are final releases.

A pre-release is a release that is not suitable for production, it is designed for testing and early feedback.
Most users should not upgrade to pre-releases as they are unstable.

You indicate a pre-release by appending two additional components to a semantic version:
`<major>.<minor>.<patch>-<label>.<number>`.
The `label` indicates which kind of pre-release this is,
you might use `alpha` and `beta` to indicate different stages of testing (and level of stability).
The `number` is used to ensure uniqueness across multiple pre-releases with the same version and label.

The version of a pre-release is determined by looking at all changes since the last **final** release.
So, if you have made a patch change since `1.2.3` and want to release an `alpha` pre-release,
the version would be `1.2.4-alpha.0`.
If you add another patch change, the next alpha version would be `1.2.4-alpha.1`.
If you then add a minor change, the next alpha version would be `1.3.0-alpha.0`.

## 0.x Versions

A major version of `0` has a special meaning, it indicates that the project is not yet stable.
You can expect packages in the `0.x` range to have [major changes](#major-changes) much more often.
This indicates to consumers that they should not use this package yet if frequently tweaking things is a problem for them.
For `0.x` versions, packages effectively only have two version components: `0.<major>.<minor>`.
We still need to clearly indicate breaking changes, so we stuff everything else into a single component.

For example, if you have a `0.1.2` version, and you make a breaking change, the next version would be `0.2.0`.
If you then make a minor change _or_ a patch change, the next version would be `0.2.1`.

If you want to go from a 0.x version to a 1.x version, see the [releasing 1.0 recipe](/recipes/releasing-100).
