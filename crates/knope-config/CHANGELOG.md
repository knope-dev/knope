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
