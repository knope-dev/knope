# BumpVersion step

Bump the version of all packages using a [Semantic Versioning] rule. At least one [package] must be defined for this step to operate on.

```admonish note
It may be easier to select the appropriate version automatically using [conventional commits]. You can do this with the [`PrepareRelease`] step instead of this one.
```

## Fields

1. `rule`: The Semantic Versioning [rule](#rules) to use.
2. `label`: Only applicable to `Pre` `rule`. The pre-release label to use.

## Examples

### Single Package Pre-Release

```toml
[package]
versioned_files = ["Cargo.toml"]

[[workflows]]
name = "pre-release"

[[workflows.steps]]
type = "BumpVersion"
rule = "Pre"
label = "rc"
```

With this particular example, running `knope pre-release` would bump the version in `Cargo.toml` using the "pre" rule and the "rc" label. So if the version _was_ `0.1.2-rc.0`, it would be bumped to `0.1.2-rc.1`.

### Multiple Packages

This step runs for each defined package independently.

```toml
[packages.knope]
versioned_files = ["knope/Cargo.toml"]

[packages.knope-utils]
versioned_files = ["knope-utils/Cargo.toml"]

[[workflows]]
name = "major"

[[workflows.steps]]
type = "BumpVersion"
rule = "Major"
```

In this example, running `knope major` would bump the version in `knope/Cargo.toml` and `knope-utils/Cargo.toml` using the "major" rule. If the versions in those files were `0.1.2` and `3.0.0` respectively, they would be bumped to `0.2.0` and `4.0.0` respectively.

## Rules

### Major

Increment the Major component of the semantic version and reset all other components (e.g. 1.2.3-rc.4 -> 2.0.0).

### Minor

Increment the Minor component of the semantic version and reset all lesser components (e.g. 1.2.3-rc.4 -> 1.3.0 ).

### Patch

Increment the Patch component of the semantic version and reset all lesser components (e.g. 1.2.3-rc.4 -> 1.2.4).

### Pre

Increment the pre-release component of the semantic version or add it if missing. You must also provide a `label` parameter to this rule which will determine the pre-release string used. For example, running this rule with the `label` "rc" would change "1.2.3-rc.4" to "1.2.3-rc.5" or "1.2.3" to "1.2.4-rc.0".

```admonish warning
Only a very specific pre-release format is supported—that is `MAJOR.MINOR.PATCH-LABEL.NUMBER`. For example, `1.2.3-rc.4` is supported, but `1.2.3-rc4` is not. `LABEL` must be specified via config  or the `--prerelease-label` option in the CLI. `NUMBER` starts at 0 and increments each time the rule is applied.
```

### Release

Remove the pre-release component of the semantic version (e.g. 1.2.3-rc.4 -> 1.2.3).

### A Note on 0.x Versions

[Semantic versioning] dictates different handling of any version which has a major component of 0 (e.g. 0.1.2). This major version should not be incremented to 1 until the project has reached a stable state. As such, it would be irresponsible (and probably incorrect) for knope to increment to version 1.0.0 the first time there is a breaking change in a 0.x project. As such, any `Major` rule applied to a 0.x project will increment the `Minor` component, and any `Minor` rule will increment the `Patch` component. This effectively means that for the version `0.1.2`:

1. The first component (`0`) is ignored
2. The second component (`1`) serves as the `Major` component, and will be incremented whenever the `Major` rule is applied.
3. The third component (`2`) serves as **both** `Minor` and `Patch` and will be incremented when either rule is applied.

If you want to go from a 0.x version to a 1.x version, you must provide the `--override-version` [command line option](../../introduction.md#cli-arguments).

## Errors

This step will fail if any of the following are true:

1. A malformed version string is found while attempting to bump. Note that only a subset of [pre-release version formats](#pre) are supported.
2. No [package] is defined missing or invalid.

[semantic versioning]: https://semver.org
[package]: ../packages.md
[conventional commits]: https://conventionalcommits.org
[`preparerelease`]: ./PrepareRelease.md
