---
default: major
---

#### The `--prerelease-label` option can only be provided after a workflow

Previously, the `--prerelease-label` CLI option was always available globally and would simply be ignored if it was not useful for the selected workflow. Now, it can only be provided _after_ the name of a workflow which can use the option (right now, only a workflow which contains a [`PrepareRelease`](https://knope-dev.github.io/knope/config/step/PrepareRelease.html) step). For example, with the default workflow, `knope release --prerelease-label="rc"` is valid, but **none of these are valid**:

- `knope --prerelease-label="rc" release`
- `knope document-change --prerelease-label="rc"`
