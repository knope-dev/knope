# BumpVersion step

Bump the version of the project in any supported formats found using a [Semantic Versioning] rule.

## Supported Formats

These are the types of files that this step knows how to search for a semantic version and bump. Dobby currently will only search the current directory for these files.

1. Cargo.toml for Rust projects
1. pyproject.toml for Python projects (using [Poetry's metadata](https://python-poetry.org))
1. package.json for Node projects

## Example

```toml
[[workflows.steps]]
type = "BumpVersion"
rule = "Pre"
value = "rc"
```

Where `rule` defines the [Semantic Versioning] rule to use and `value` is optional depending on the `rule`.

## Rules

### Major

Increment the Major component of the semantic version and reset all other components (e.g. 1.2.3-rc.4 -> 2.0.0).

### Minor

Increment the Minor component of the semantic version and reset all lesser components (e.g. 1.2.3-rc.4 -> 1.3.0 ).

### Patch

Increment the Patch component of the semantic version and reset all lesser components (e.g. 1.2.3-rc.4 -> 1.2.4).

### Pre

Increment the pre-release component of the semantic version or add it if missing. You must also provide a `value` parameter to this rule which will determine the pre-release string used. For example, running this rule with the `value` "rc" would change "1.2.3-rc.4" to "1.2.3-rc.5" or "1.2.3" to "1.2.3-rc.0".

### A Note on 0.x Versions

[Semantic versioning] dictates different handling of any version which has a major component of 0 (e.g. 0.1.2). This major version should not be incremented to 1 until the project has reached a stable state. As such, it would be irresponsible (and probably incorrect) for Dobby to increment to version 1.0.0 the first time there is a breaking change in a 0.x project. As such, any `Major` rule applied to a 0.x project will increment the `Minor` component, and any `Minor` rule will increment the `Patch` component. This effectively means that for the version `0.1.2`:

1. The first component (`0`) is ignored
2. The second component (`1`) serves as the `Major` component, and will be incremented whenever the `Major` rule is applied.
3. The third component (`2`) serves as **both** `Minor` and `Patch` and will be incremented when either rule is applied.

### Release

Remove the pre-release component of the semantic version (e.g. 1.2.3-rc.4 -> 1.2.3).

## Errors

This step will fail if any of the following are true:

1. A malformed version string is found while attempting to bump.

[semantic versioning]: https://semver.org
