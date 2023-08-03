---
default: minor
---

#### Added the `--override-version` option to manually set the next version

Allows you to manually determine the next version for a [`BumpVersion`] or [`PrepareRelease`] instead of using a semantic versioning rule. This option can only be provided after a workflow which contains a relevant step. This has two formats, depending on whether there is [one package](https://knope-dev.github.io/knope/config/packages.html#a-single-package-with-a-single-versioned-file) or [multiple packages](https://knope-dev.github.io/knope/config/packages.html#multiple-packages):
1. `--override-version 1.0.0` will set the version to `1.0.0` if there is only one package configured (error if multiple packages are configured).
2. `--override-version first-package=1.0.0 --override-version second-package=2.0.0` will set the version of `first-package` to `1.0.0` and `second-package` to `2.0.0` if there are multiple packages configured (error if only one package is configured).
