---
default: minor
---

#### New `CreatePullRequest` step

The new [`CreatePullRequest` step] allows you to create or update a pull request on GitHub. It's designed to be a nice way to preview and accept new releases via a pull request workflow, but could certainly work for more contexts as well! To see an example of the new PR-based release workflow, check out [Knope's prepare-release workflow] and [Knope's release workflow].

[`CreatePullRequest` step]: https://knope-dev.github.io/knope/docs/config/step/CreatePullRequest
[Knope's prepare-release workflow]: https://github.com/knope-dev/knope/blob/e7292fa746fe1d81b84e5848815c02a0d8fc6f95/.github/workflows/prepare_release.yml
[knope's release workflow]: https://github.com/knope-dev/knope/blob/e7292fa746fe1d81b84e5848815c02a0d8fc6f95/.github/workflows/release.yml
