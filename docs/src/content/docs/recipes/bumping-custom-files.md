---
title: Versioning unsupported files
---

Want to bump the version of a file that isn't [natively supported](/reference/config-file/packages#versioned_files)?
[Request it as a feature] so it can be added to Knope's built-in support! In the meantime, you have a couple options:

## Using regex patterns

Knope can update the version of _any_ file using regex patterns:

```toml
[package]
versioned_files = [
    { path = "README.md", regex = "v(?<version>\\d+\\.\\d+\\.\\d+)" }
]
```

The regex pattern **must** include a named capture group `(?<version>...)` around the version number you want to replace.

If you need to update multiple version strings with different formats in the same file, you can provide an array of patterns:

```toml
[package]
versioned_files = [
    { path = "config.json", regex = [
        '"version": "(?<version>\\d+\\.\\d+\\.\\d+)"',
        'image: app:v(?<version>\\d+\\.\\d+\\.\\d+)'
    ]}
]
```

See the [Text files with regex patterns][regex] section for more details.

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

## Version a custom script

For more advanced cases, you can combine the `regex` feature with a [`Command`] step:

```sh title = "custom-steps.py"
version = "1.2.3"

# ... do custom things with the version
```

```toml title = "knope.toml"
[package]
versioned_files = [
    { path = "custom-steps.py", regex = 'version = \"(?<version>.*)\"' }
]

[[workflows]]
name = "release"

[[workflows.steps]]
# This updates the version in `custom-steps.py`
type = "PrepareRelease"

[[workflows.steps]]
type = "Command"
command = "python custom-steps.py"
```

[request it as a feature]: https://github.com/knope-dev/knope/issues
[`bumpversion`]: /reference/config-file/steps/bump-version
[`preparerelease`]: /reference/config-file/steps/prepare-release
[`release`]: /reference/config-file/steps/release
[`command`]: /reference/config-file/steps/command
[regex]: /reference/config-file/packages#regex-patterns
