---
versioning: major
config: major
---

# New APIs for internal dependency propagation

To support automatic monorepo dependency updates:

- `knope-versioning`: `ChangeSource` has a new `DependencyUpdate` variant (breaking for exhaustive matches). `Package::versioned_files()`, `Config::is_lock_file()`, `VersionedFile::package_name()`, and `VersionedFile::declares_dependency()` were added.
- `knope-config`: `Package` has new `update_internal_dependencies` and `internal_dependencies` fields (breaking for struct-literal construction), and the `InternalDependencyUpdate` enum was added.
