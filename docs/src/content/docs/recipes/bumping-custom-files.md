---
title: Versioning unsupported files
---

Want to bump the version of a file that isn't [natively supported](/reference/config-file/packages#versioned_files)?
[Request it as a feature] and, in the meantime, you can write a script to manually bump that file with the version
produced by [`BumpVersion`] or [`PrepareRelease`] using a [`Command`] step, like this:

```toml
[package]
versioned_files = []  # With no versioned_files, the version will be determined via Git tag
changelog = "CHANGELOG.md"

[[workflows]]
name = "release"

[[workflows.steps]]
type = "PrepareRelease"

[[workflows.steps]]
type = "Command"
command = "my-command-which-bumps-a-custom-file-with version"
variables = { "version" = "Version" }
```

:::caution
The `Version` variable in the [`Command`] step can't be used when multiple packages are defined.
This is a temporary limitation—if you have a specific use case for this, please [file an issue][request it as a feature].
:::

[request it as a feature]: https://github.com/knope-dev/knope/issues
[`bumpversion`]: /reference/config-file/steps/bump-version
[`preparerelease`]: /reference/config-file/steps/prepare-release
[`release`]: /reference/config-file/steps/release
[`command`]: /reference/config-file/steps/command
