## 0.3.0 (2024-08-18)

### Breaking Changes

#### Support for dependencies within `Cargo.toml`

Dependencies within a `Cargo.toml` file [can now be updated](https://knope.tech/reference/config-file/packages/)
as part of `versioned_files`.

### Features

#### Support for `Cargo.lock` in `versioned_files`

Dependencies within a `Cargo.lock` [can now be updated](https://knope.tech/reference/config-file/packages#cargolock).

## 0.2.0 (2024-08-10)

### Breaking Changes

- Move HeaderLevel to internal, parse with Changelog::new

### Features

- `impl From<ReleaseTag> for String`

## 0.1.0 (2024-08-04)

### Breaking Changes

#### Everything has changed

And hopefully it won't break anyone since this crate isn't ready for external consumers yet!

### Features

- Add handling of changes (conventional commits, changesets)

## 0.0.1 (2024-04-14)

### Features

- Initial release
