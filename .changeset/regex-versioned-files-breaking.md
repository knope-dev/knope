---
knope-versioning: major
knope-config: major
---

**Breaking Changes for library users:**
- `knope-versioning::VersionedFileConfig::new()` now takes an additional `regex: Option<String>` parameter
- `knope-versioning::VersionedFileConfig::new()` now returns `Result<Self, ConfigError>` instead of `Result<Self, UnknownFile>`
- `knope-config::VersionedFile::TryFrom` now returns `ConfigError` instead of `UnknownFile`
