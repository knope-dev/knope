---
default: minor
---

#### Added an `allow_empty` option to the `PrepareRelease` step

Closes #416

If you want to run `PrepareRelease` on every push to a branch without it failing when there's nothing to release, you can now include the `allow_empty` option like this:

```toml
[[workflows.steps]]
type = "PrepareRelease"
allow_empty = true
```

Then, you can use some logic to gracefully skip the rest of your CI process if there is nothing to release. For example, in GitHub Actions, you could do something like this:

```yaml
- name: Prepare Release
  run: knope prepare-release
- name: Check for Release
  id: status
  run: echo ready=$(if [[ `git status --porcelain` ]]; then echo "true"; else echo "false"; fi;) >> $GITHUB_OUTPUT
- name: Release
  if: steps.status.outputs.ready == 'true'
  run: knope release
```

This allows you to differentiate between there being nothing to release and the `PrepareRelease` step failing for other reasons.
