# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.2.0

### Breaking Changes

- Rename to Knope, which has much more positive associations. (#161)
- Allow switching between pre-release prefixes instead of erroring (e.g. -alpha.1 -> -beta.0)
- `BumpVersion` now takes a `label` parameter for the `Pre` rule instead of `value`.
- `UpdateProjectFromCommits` step has been renamed to `PrepareRelease`.

### Features

- Add a `--generate` option for generating a brand-new config file with a default `release` workflow. (#159)
- Add top-level `--validate` and per-workflow `--dry-run` options. (#158)
- Add a `dry-run` option to the `PrepareRelease` step. (#139, #137)
- Add a `Release` step for generating GitHub releases. (#136)
- Support pre-releases in `UpdateProjectFromCommits`. (#132)

### Fixes

- update rust crate git2 to 0.14.2 (#157)
- Bump version before adding a pre-release component.
- Stop parsing Markdown in Changelogs to avoid errors in unimplemented features. (#127)

## 0.1.5

### Fixes

- Properly handle Windows newlines in commits (#119)

## 0.1.4

### Features

- Support the BREAKING CHANGE footer with a separate breaking description.

### Fixes

- update rust crate dialoguer to 0.9.0 and console to 0.15.0 (#114)

## 0.1.3

### Features

- You can now pass the name of a workflow as an argument to bypass the selection prompt (closes #24)

### Fixes

- Commits with extra whitespace at the end were not being recorded properly

## 0.1.2

### Fixes

- Retain property order when writing changes to package.json (#75)

## 0.1.1

### Features

- Specify a path to a changelog file in UpdateProjectFromCommits (closes #27) (#71)
- Use special version bumping rules for versions that start with 0.x (closes #37) (#65)

## 0.1.0 - 2021-01-29

- Initial release
