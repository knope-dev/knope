## 0.4.2 (2026-02-16)

### Features

- Build binaries for ARM Linux (#1772)

## 0.4.1 (2026-02-03)

### Features

#### Add `[changes]` config section with `ignore_conventional_commits` setting

Adds a new top-level `[changes]` configuration section to control how Knope processes changes. The first setting in this section is `ignore_conventional_commits`, which when set to `true`, makes Knope ignore conventional commits and only use changesets for determining version bumps and changelog entries.

This replaces the deprecated step-level `ignore_conventional_commits` option on the `PrepareRelease` step. Use `knope --upgrade` to automatically migrate from the old format to the new one.

**Example configuration:**

```toml
[changes]
ignore_conventional_commits = true

[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"
```

See the [changes config documentation](https://knope.tech/reference/config-file/changes) for more details.

## 0.4.0 (2026-01-20)

### Breaking Changes

- Add support for [multiple regex patterns](https://knope.tech/reference/config-file/packages/#multiple-patterns) when versioning a single file

## 0.3.2 (2026-01-12)

### Notes

- Bump dependencies

## 0.3.1 (2025-12-02)

### Fixes

- #1678 cant have a prerelease label with a 'v' in it. (#1680)

## 0.3.0 (2025-11-03)

### Breaking Changes

- `VersionedFile::TryFrom` now returns `ConfigError` instead of `UnknownFile`

## 0.2.5 (2025-08-10)

### Features

- New `Config` and `ReleaseNotes` structs

## 0.2.4 (2025-06-23)

### Fixes

- Update relative-path dependency

## 0.2.3 (2025-05-03)

### Features

- Add Variable and Template structs (from CLI)

## 0.2.2 (2025-04-05)

### Features

- Print each step before it runs when `--verbose` is set (#1399)

## 0.2.1 (2025-03-08)

### Notes

- Update to Rust edition 2024 and MSRV 1.85

## 0.2.0 (2024-09-15)

### Breaking Changes

- Changed type of `Package::assets` to `Assets` enum

## 0.1.0 (2024-08-18)

### Breaking Changes

#### Support for dependencies within `Cargo.toml`

Dependencies within a `Cargo.toml` file [can now be updated](https://knope.tech/reference/config-file/packages/)
as part of `versioned_files`.

## 0.0.1 (2024-08-04)

### Features

- Initial release
