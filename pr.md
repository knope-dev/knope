## Summary

Add support for multiple regex patterns when versioning a single file. This allows users to update multiple version strings with different formats in the same file.

### Problem

Previously, the `regex` option for `versioned_files` only accepted a single pattern. If a user needed to match multiple version formats in the same file, they might try adding multiple `versioned_files` entries for the same path with different patterns. However, this did not work because each entry kept a separate copy of the file content, so only the last entry's changes would be applied, silently overwriting any earlier updates.

### Solution

The `regex` field now accepts either:
- A single string (existing behavior): `regex = "v(?<version>\\d+\\.\\d+\\.\\d+)"`
- An array of strings (new): `regex = ["pattern1", "pattern2"]`

When multiple patterns are provided:
- All patterns must match for version detection to succeed
- All matching patterns are updated when the version changes

### Example

```toml
[package]
versioned_files = [
    { path = "config.json", regex = [
        '"version": "(?<version>\d+\.\d+\.\d+)"',
        'image: app:v(?<version>\d+\.\d+\.\d+)'
    ]}
]
```

## Test plan

- [x] Existing regex tests pass
- [x] New test `test_multiple_regexes_same_file` verifies multiple patterns work correctly
- [x] Clippy passes with no warnings
- [x] Documentation updated in `packages.mdx` and `bumping-custom-files.md`
