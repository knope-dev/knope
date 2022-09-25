# Default Workflows

Knope can do a lot out of the box, with no need for a [config file](config/config.md). If no file is found, Knope will use the same config as it would create with `knope --generate`.

```admonish warning
If you have a `knope.toml` file in your project, this page is no longer relevant to you, as default workflows are only used when _no_ config file is found.
```

## `release`

Without any config, you can run `knope release` to create a new release from [conventional commits]. This will:

1. Update the version in any [supported package files](config/packages.md#versioned_files) based on the [semantic version] determined from all commits since the last release. For more detail, see the [`PrepareRelease`] step.
2. Update a `CHANGELOG.md` file (if any) with the body of relevant commits (again, see the [`PrepareRelease`] step for more detail).
3. Commit the changes to the versioned and changelog files and push that commit.
4. Create a release which is one of the following, see the [`Release`] step for more detail:
   1. If your remote is GitHub, create a new release on GitHub with the same body as the changelog entry. **This requires a `GITHUB_TOKEN` environment variable to be set.**
   2. If the remote is not GitHub, a tag will be created and pushed to the remote.

### Additional Options

1. `--dry-run` will run the workflow without modifying any files or interacting with the remote. Instead, all the steps that _would_ happen will be printed to the screen so you can verify what will happen.
2. `--prerelease-label` will tell `knope` to create a prerelease with a given label. For example, `knope release --prerelease-label rc` will create a release with the _next_ calculated version (as if you had run `knope release`), but with the `-rc.0` suffix (or `rc.1`, `rc.2`, etc. if you have already created a release with that label).

[conventional commits]: https://www.conventionalcommits.org/en/v1.0.0/
[semantic version]: https://semver.org
[`PrepareRelease`]: config/step/PrepareRelease.md
[`Release`]: config/step/Release.md
