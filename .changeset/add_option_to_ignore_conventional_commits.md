---
knope: minor
---

# Add option to ignore conventional commits

You can now add `ignore_conventional_commits = true` to a [`PrepareRelease` step](https://knope.tech/reference/config-file/steps/prepare-release/)
to ignore commit messages (and only consider changesets):

```toml
[[workflows.steps]]
type = "PrepareRelease"
ignore_conventional_commits = true
```

PR #1008 closes #924. Thanks for the suggestion @ematipico!
