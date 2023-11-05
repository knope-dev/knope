---
title: Releasing 1.0.0
---

Releasing version `1.0.0` of a package with Knope is a bit tricky.
If you were on a `0.x` version,
[Knope will never select `1.0` for you](/reference/concepts/semantic-versioning#0x-versions).
However,
you can always override the version that Knope selects for a [`PrepareRelease`] or [`BumpVersion`] step using the [`--override-version` command line argument].

For example, using the [default workflows],
the `knope release` command uses the [`PrepareRelease`] step to determine the next version of your package.
If we run `knope release --override-version 1.0.0`,
the version selected will be `1.0.0` regardless of which changes were included in the release.

[`PrepareRelease`]: /reference/config-file/steps/prepare-release
[`BumpVersion`]: /reference/config-file/steps/bump-version
[`--override-version` command line argument]: /reference/command-line-arguments#--override-version
[default workflows]: /reference/default-workflows
