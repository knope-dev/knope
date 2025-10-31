---
title: Versioning unsupported files
---

Want to bump the version of a file that isn't [natively supported](/reference/config-file/packages#versioned_files)?

[Request it as a feature] so it can be added to Knope's built-in support! In the meantime, you have a couple options:

## Using a custom script

You can write a script to manually bump that file with the version produced by [`BumpVersion`] or [`PrepareRelease`] using a [`Command`] step, like this:

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
command = "my-command-which-bumps-a-custom-file-with $version"
```

:::caution
The `Version` variable in the [`Command`] step, including the default `$version`, can't be used when multiple packages are defined.
This is a temporary limitation—if you have a specific use case for this, please [file an issue][request it as a feature].
:::

## Using regex patterns

For text files like README.md or documentation where a simple pattern can match the version, you can use regex patterns to find and replace version strings:

```toml
[package]
versioned_files = [
    "Cargo.toml",  # Your main versioned file
    { path = "README.md", regex = "v(?<version>\\d+\\.\\d+\\.\\d+)" }
]
```

The regex pattern must include a named capture group `(?<version>...)` around the version number you want to replace. See the [Text files with regex patterns](/reference/config-file/packages#text-files-with-regex-patterns) section for more details.

[request it as a feature]: https://github.com/knope-dev/knope/issues
[`bumpversion`]: /reference/config-file/steps/bump-version
[`preparerelease`]: /reference/config-file/steps/prepare-release
[`release`]: /reference/config-file/steps/release
[`command`]: /reference/config-file/steps/command
