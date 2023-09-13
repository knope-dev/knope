---
default: patch
---

#### Consistent commit selection in branching histories

PR #574 fixes issue #505 from @BatmanAoD.

Previous versions of Knope did not handle branching histories correctly. In some cases, this could result in commits from previous stable releases being included in a new release. It could _also_ result in missing some commits that _should_ have been included. This has been fixed—Knope should provide you the same commit list that `git rev-list {previous_stable_tag}..HEAD` would.
