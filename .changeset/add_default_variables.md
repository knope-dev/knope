---
knope: major
---

# Add default variables

[Default variables](https://knope.tech/reference/config-file/variables/#defaults) will now apply anywhere they can be
used—including the `Command` and `CreatePullRequest` steps.

If any of the defaults, like `$version` or `$changelog`, appear in a variable-supporting location
and you don't have explicit `variables =` set for that step, this is a breaking change.
