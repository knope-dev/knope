---
default: minor
---

#### Report files to be added to git in `--dry-run`

The [`PrepareRelease`](https://knope-dev.github.io/knope/config/step/PrepareRelease.html) adds modified files to Git. Now, when running with the `--dry-run` option, it will report which files would be added to Git (for easier debugging).

> Note: The default `knope release` workflow includes this [`PrepareRelease`] step.
