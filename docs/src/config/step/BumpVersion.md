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

### Release

Remove the pre-release component of the semantic version (e.g. 1.2.3-rc.4 -> 1.2.3).

## Errors

This step will fail if any of the following are true:

1. A malformed version string is found while attempting to bump.

[semantic versioning]: https://semver.org
