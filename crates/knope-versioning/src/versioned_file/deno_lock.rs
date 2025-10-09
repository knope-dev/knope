use std::{
    borrow::ToOwned,
    collections::{HashSet, VecDeque},
    error::Error as StdError,
    fmt::Write as _,
};

use async_trait::async_trait;
use deno_lockfile::{
    DeserializationError as LockfileDeserializationError, Lockfile, Lockfile5NpmInfo,
    LockfileContent, LockfileError, LockfileErrorReason, NewLockfileOptions,
    NpmPackageInfoProvider,
};
use deno_semver::{
    Version as DenoVersion,
    jsr::JsrDepPackageReq,
    package::{PackageKind, PackageNv},
};
use futures::executor::block_on;
use relative_path::RelativePathBuf;
use serde_json::Value;
use thiserror::Error;

use crate::{action::Action, semver::Version};

type JsonObject = serde_json::Map<String, Value>;

#[derive(Clone, Debug)]
pub struct DenoLock {
    path: RelativePathBuf,
    json: JsonObject,
    content: LockfileContent,
    diff: Option<String>,
}

impl DenoLock {
    pub(crate) fn new(path: RelativePathBuf, content: &str) -> Result<Self, Error> {
        let json_value: Value =
            serde_json::from_str(content).map_err(|source| Error::Deserialize {
                path: path.clone(),
                source,
            })?;

        let Value::Object(initial_map) = json_value else {
            return Err(Error::UnexpectedStructure(path));
        };

        let version = initial_map
            .get("version")
            .and_then(Value::as_str)
            .map(str::to_owned);

        let version_for_error = version.clone().unwrap_or_else(|| "1".to_string());
        match version.as_deref() {
            Some("5") => {}
            Some("4" | "3" | "2" | "1") | None => {
                return Err(Error::LegacyLockfileNeedsNpmInfo {
                    path: path.clone(),
                    version: version_for_error,
                });
            }
            Some(other) => {
                return Err(Error::UnsupportedVersion {
                    path: path.clone(),
                    version: other.to_string(),
                });
            }
        }

        let lockfile = block_on(Lockfile::new(
            NewLockfileOptions {
                file_path: path.to_path(""),
                content,
                overwrite: false,
            },
            &NoNpmInfoProvider,
        ))
        .map_err(|err| map_lockfile_error(*err, &path, version.clone()))?;

        let canonical_value: Value =
            serde_json::from_str(&lockfile.as_json_string()).map_err(|source| {
                Error::Deserialize {
                    path: path.clone(),
                    source,
                }
            })?;

        let Value::Object(map) = canonical_value else {
            return Err(Error::UnexpectedStructure(path));
        };

        Ok(Self {
            path,
            json: map,
            content: lockfile.content,
            diff: None,
        })
    }

    pub(crate) fn get_path(&self) -> &RelativePathBuf {
        &self.path
    }

    pub(crate) fn set_version(
        mut self,
        new_version: &Version,
        dependency: Option<&str>,
    ) -> Result<Self, Error> {
        let Some(dependency_name) = dependency else {
            return Ok(self);
        };

        let Some(Value::Object(specifiers)) = self.json.get_mut("specifiers") else {
            return Ok(self);
        };

        let mut old_versions = HashSet::new();
        let mut kinds = HashSet::new();
        let mut updated = false;

        for (specifier, value) in specifiers.iter_mut() {
            let Ok(dep_req) = JsrDepPackageReq::from_str(specifier) else {
                continue;
            };

            if dep_req.req.name.as_str() != dependency_name {
                continue;
            }

            let Value::String(resolved) = value else {
                continue;
            };
            old_versions.insert(resolved.clone());
            old_versions.insert(dep_req.req.version_req.version_text().to_string());
            kinds.insert(dep_req.kind);
            *resolved = new_version.to_string();
            updated = true;
        }

        if !updated {
            return Ok(self);
        }

        let resolved_version = new_version.to_string();
        let deno_version = DenoVersion::parse_standard(&resolved_version).map_err(|source| {
            Error::InvalidVersion {
                version: resolved_version.clone(),
                source,
            }
        })?;

        if kinds.contains(&PackageKind::Jsr) {
            update_registry_packages(
                &mut self.json,
                "jsr",
                dependency_name,
                &old_versions,
                &deno_version,
                &resolved_version,
            );
        }

        if kinds.contains(&PackageKind::Npm) {
            update_registry_packages(
                &mut self.json,
                "npm",
                dependency_name,
                &old_versions,
                &deno_version,
                &resolved_version,
            );
        }

        update_workspace(
            self.json
                .get_mut("workspace")
                .and_then(Value::as_object_mut),
            dependency_name,
            &old_versions,
            &resolved_version,
        );

        self.content =
            LockfileContent::from_json(Value::Object(self.json.clone())).map_err(|source| {
                Error::LockfileDeserialize {
                    path: self.path.clone(),
                    source: Box::new(source),
                }
            })?;

        let diff = self.diff.get_or_insert_with(String::new);
        if !diff.is_empty() {
            diff.push_str(", ");
        }
        write!(diff, "{dependency_name} = {resolved_version}").ok();

        Ok(self)
    }

    pub(crate) fn write(self) -> Option<Action> {
        self.diff.map(|diff| {
            let lockfile = Lockfile {
                overwrite: false,
                has_content_changed: true,
                content: self.content,
                filename: self.path.to_path(""),
            };
            Action::WriteToFile {
                path: self.path,
                content: lockfile.as_json_string(),
                diff,
            }
        })
    }
}

fn map_lockfile_error(
    error: LockfileError,
    path: &RelativePathBuf,
    version: Option<String>,
) -> Error {
    match error.source {
        LockfileErrorReason::Empty => Error::UnexpectedStructure(path.clone()),
        LockfileErrorReason::ParseError(source) => Error::Deserialize {
            path: path.clone(),
            source,
        },
        LockfileErrorReason::UnsupportedVersion {
            version: lock_version,
        } => Error::UnsupportedVersion {
            path: path.clone(),
            version: lock_version,
        },
        LockfileErrorReason::DeserializationError(source) => Error::LockfileDeserialize {
            path: path.clone(),
            source: Box::new(source),
        },
        LockfileErrorReason::TransformError(transform_error) => {
            if transform_error
                .source()
                .and_then(|inner| inner.downcast_ref::<MissingNpmInfo>())
                .is_some()
            {
                Error::LegacyLockfileNeedsNpmInfo {
                    path: path.clone(),
                    version: version.unwrap_or_else(|| "unknown".to_string()),
                }
            } else {
                Error::Transform {
                    path: path.clone(),
                    source: Box::new(transform_error),
                }
            }
        }
    }
}

fn update_registry_packages(
    root: &mut JsonObject,
    section: &str,
    dependency_name: &str,
    old_versions: &HashSet<String>,
    new_version: &DenoVersion,
    resolved_version: &str,
) {
    let Some(Value::Object(section_map)) = root.get_mut(section) else {
        return;
    };

    let mut replacements = Vec::new();
    for key in section_map.keys() {
        let Ok(nv) = PackageNv::from_str(key) else {
            continue;
        };
        if nv.name.as_str() == dependency_name && old_versions.contains(&nv.version.to_string()) {
            replacements.push(key.clone());
        }
    }

    for key in replacements {
        if let Some(value) = section_map.remove(&key) {
            let new_key = if let Ok(mut nv) = PackageNv::from_str(&key) {
                nv.version = new_version.clone();
                nv.to_string()
            } else {
                format!("{dependency_name}@{resolved_version}")
            };
            section_map.insert(new_key, value);
        }
    }
}

fn update_workspace(
    workspace: Option<&mut JsonObject>,
    dependency_name: &str,
    old_versions: &HashSet<String>,
    resolved_version: &str,
) {
    let Some(workspace) = workspace else {
        return;
    };

    update_dependency_arrays(workspace, dependency_name, old_versions, resolved_version);

    if let Some(Value::Object(members)) = workspace.get_mut("members") {
        for member in members.values_mut() {
            if let Value::Object(member_obj) = member {
                update_dependency_arrays(
                    member_obj,
                    dependency_name,
                    old_versions,
                    resolved_version,
                );
            }
        }
    }

    if let Some(Value::Object(links)) = workspace.get_mut("links") {
        let mut key_updates = VecDeque::new();
        let keys: Vec<String> = links.keys().cloned().collect();
        for key in keys {
            if let Some(new_key) =
                updated_specifier(&key, dependency_name, old_versions, resolved_version)
            {
                if let Some(value) = links.remove(&key) {
                    key_updates.push_back((new_key, value));
                }
            }
        }
        while let Some((new_key, mut value)) = key_updates.pop_front() {
            if let Value::Object(link_obj) = &mut value {
                update_dependency_arrays(link_obj, dependency_name, old_versions, resolved_version);
            }
            links.insert(new_key, value);
        }
    }
}

fn update_dependency_arrays(
    object: &mut JsonObject,
    dependency_name: &str,
    old_versions: &HashSet<String>,
    resolved_version: &str,
) {
    if let Some(Value::Array(array)) = object.get_mut("dependencies") {
        update_specifier_array(array, dependency_name, old_versions, resolved_version);
    }
    if let Some(Value::Object(package_json)) = object.get_mut("packageJson") {
        if let Some(Value::Array(array)) = package_json.get_mut("dependencies") {
            update_specifier_array(array, dependency_name, old_versions, resolved_version);
        }
    }
    if let Some(Value::Array(optional)) = object.get_mut("optionalDependencies") {
        update_specifier_array(optional, dependency_name, old_versions, resolved_version);
    }
    if let Some(Value::Array(peers)) = object.get_mut("peerDependencies") {
        update_specifier_array(peers, dependency_name, old_versions, resolved_version);
    }
}

fn update_specifier_array(
    array: &mut [Value],
    dependency_name: &str,
    old_versions: &HashSet<String>,
    resolved_version: &str,
) {
    for value in array.iter_mut() {
        if let Value::String(text) = value {
            if let Some(new_text) =
                updated_specifier(text, dependency_name, old_versions, resolved_version)
            {
                *text = new_text;
            }
        }
    }
}

fn updated_specifier(
    text: &str,
    dependency_name: &str,
    old_versions: &HashSet<String>,
    resolved_version: &str,
) -> Option<String> {
    let dep_req = JsrDepPackageReq::from_str(text).ok()?;
    if dep_req.req.name.as_str() != dependency_name {
        return None;
    }
    if !old_versions.contains(dep_req.req.version_req.version_text()) {
        return None;
    }
    Some(format!(
        "{}{}@{}",
        dep_req.kind.scheme_with_colon(),
        dependency_name,
        resolved_version
    ))
}

struct NoNpmInfoProvider;

#[derive(Debug, Error)]
#[error("Lockfile conversion requires npm metadata")]
struct MissingNpmInfo;

#[async_trait(?Send)]
impl NpmPackageInfoProvider for NoNpmInfoProvider {
    async fn get_npm_package_info(
        &self,
        _values: &[PackageNv],
    ) -> Result<Vec<Lockfile5NpmInfo>, Box<dyn StdError + Send + Sync>> {
        Err(Box::new(MissingNpmInfo))
    }
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "miette", derive(miette::Diagnostic))]
pub enum Error {
    #[error("Error deserializing {path}: {source}")]
    Deserialize {
        path: RelativePathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Unsupported lockfile version {version} in {path}")]
    UnsupportedVersion {
        path: RelativePathBuf,
        version: String,
    },
    #[error(
        "Lockfile version {version} in {path} requires npm package metadata which is not supported yet"
    )]
    LegacyLockfileNeedsNpmInfo {
        path: RelativePathBuf,
        version: String,
    },
    #[error("Error transforming legacy lockfile {path}: {source}")]
    Transform {
        path: RelativePathBuf,
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },
    #[error("Error deserializing lockfile content {path}: {source}")]
    LockfileDeserialize {
        path: RelativePathBuf,
        #[source]
        source: Box<LockfileDeserializationError>,
    },
    #[error("Lockfile {0} did not contain valid Deno packages structure")]
    UnexpectedStructure(RelativePathBuf),
    #[error("Invalid version '{version}' for Deno dependency: {source}")]
    InvalidVersion {
        version: String,
        #[source]
        source: deno_semver::VersionParseError,
    },
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn sample_lock() -> String {
        r#"{
  "version": "5",
  "specifiers": {
    "jsr:@scope/first@^1.0.0": "1.0.0",
    "npm:left-pad@^1": "1.3.0"
  },
  "jsr": {
    "@scope/first@1.0.0": {
      "integrity": "sha256-abc",
      "dependencies": []
    }
  },
  "npm": {
    "left-pad@1.3.0": {
      "integrity": "sha256-def",
      "dependencies": []
    }
  },
  "redirects": {},
  "remote": {},
  "workspace": {
    "dependencies": [
      "jsr:@scope/first@1.0.0",
      "npm:left-pad@1.3.0"
    ],
    "packageJson": {
      "dependencies": [
        "jsr:@scope/first@^1.0.0",
        "npm:left-pad@^1"
      ]
    },
    "members": {
      "first": {
        "dependencies": [
          "jsr:@scope/first@1.0.0",
          "npm:left-pad@1.3.0"
        ],
        "packageJson": {
          "dependencies": [
            "jsr:@scope/first@^1.0.0",
            "npm:left-pad@^1"
          ]
        }
      }
    },
    "links": {
      "jsr:@scope/first@1.0.0": {
        "dependencies": [],
        "optionalDependencies": [],
        "peerDependencies": [],
        "peerDependenciesMeta": {}
      },
      "npm:left-pad@1.3.0": {
        "dependencies": [],
        "optionalDependencies": [
          "npm:left-pad@1.3.0"
        ],
        "peerDependencies": [
          "npm:left-pad@^1"
        ],
        "peerDependenciesMeta": {}
      }
    }
  }
}"#
        .to_string()
    }

    #[test]
    fn updates_specifiers_and_workspace() {
        let content = sample_lock();
        let lock = DenoLock::new(RelativePathBuf::from("deno.lock"), &content).unwrap();
        let updated = lock
            .set_version(&Version::new(1, 2, 0, None), Some("@scope/first"))
            .unwrap();
        let write_action = updated.write().unwrap();
        if let Action::WriteToFile { content, diff, .. } = write_action {
            assert!(content.contains("\"jsr:@scope/first@1\": \"1.2.0\""));
            assert!(content.contains("\"@scope/first@1.2.0\""));
            assert!(content.contains("\"jsr:@scope/first@1.2.0\""));
            assert_eq!(diff, "@scope/first = 1.2.0");
        } else {
            panic!("Expected write action");
        }
    }

    #[test]
    fn ignores_missing_dependency() {
        let content = sample_lock();
        let lock = DenoLock::new(RelativePathBuf::from("deno.lock"), &content).unwrap();
        let lock = lock
            .set_version(&Version::new(1, 2, 0, None), Some("@scope/second"))
            .unwrap();
        assert!(lock.write().is_none());
    }

    #[test]
    fn updates_npm_dependency() {
        let content = sample_lock();
        let lock = DenoLock::new(RelativePathBuf::from("deno.lock"), &content).unwrap();
        let updated = lock
            .set_version(&Version::new(1, 4, 0, None), Some("left-pad"))
            .unwrap();
        let write_action = updated.write().unwrap();
        if let Action::WriteToFile { content, diff, .. } = write_action {
            let value: Value = serde_json::from_str(&content).unwrap();
            let specifiers = value
                .get("specifiers")
                .and_then(Value::as_object)
                .expect("specifiers map");
            assert_eq!(
                specifiers
                    .get("npm:left-pad@1")
                    .and_then(Value::as_str)
                    .unwrap(),
                "1.4.0"
            );
            let npm = value
                .get("npm")
                .and_then(Value::as_object)
                .expect("npm map");
            assert!(npm.contains_key("left-pad@1.4.0"));
            let workspace = value
                .get("workspace")
                .and_then(Value::as_object)
                .expect("workspace map");
            let workspace_deps = workspace
                .get("dependencies")
                .and_then(Value::as_array)
                .expect("workspace dependencies");
            assert!(
                workspace_deps
                    .iter()
                    .any(|value| value.as_str() == Some("npm:left-pad@1.4.0"))
            );
            let package_json_deps = workspace
                .get("packageJson")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("dependencies"))
                .and_then(Value::as_array)
                .expect("packageJson dependencies");
            assert!(
                package_json_deps
                    .iter()
                    .any(|value| value.as_str() == Some("npm:left-pad@1.4.0"))
            );
            let members = workspace
                .get("members")
                .and_then(Value::as_object)
                .expect("members map");
            let first = members
                .get("first")
                .and_then(Value::as_object)
                .expect("first member");
            let first_deps = first
                .get("dependencies")
                .and_then(Value::as_array)
                .expect("first dependencies");
            assert!(
                first_deps
                    .iter()
                    .any(|value| value.as_str() == Some("npm:left-pad@1.4.0"))
            );
            let first_package_json_deps = first
                .get("packageJson")
                .and_then(Value::as_object)
                .and_then(|obj| obj.get("dependencies"))
                .and_then(Value::as_array)
                .expect("first packageJson dependencies");
            assert!(
                first_package_json_deps
                    .iter()
                    .any(|value| value.as_str() == Some("npm:left-pad@1.4.0"))
            );
            let links = workspace
                .get("links")
                .and_then(Value::as_object)
                .expect("links map");
            let npm_link = links
                .get("npm:left-pad@1.4.0")
                .and_then(Value::as_object)
                .expect("npm link");
            let optional_deps = npm_link
                .get("optionalDependencies")
                .and_then(Value::as_array)
                .expect("optional dependencies");
            assert!(
                optional_deps
                    .iter()
                    .any(|value| value.as_str() == Some("npm:left-pad@1.4.0"))
            );
            let peer_deps = npm_link
                .get("peerDependencies")
                .and_then(Value::as_array)
                .expect("peer dependencies");
            assert!(
                peer_deps
                    .iter()
                    .any(|value| value.as_str() == Some("npm:left-pad@1.4.0"))
            );
            assert_eq!(diff, "left-pad = 1.4.0");
        } else {
            panic!("Expected write action");
        }
    }

    #[test]
    fn legacy_version_errors() {
        let content = r#"{"version":"4"}"#.to_string();
        let err = DenoLock::new(RelativePathBuf::from("deno.lock"), &content).unwrap_err();
        assert!(matches!(err, Error::LegacyLockfileNeedsNpmInfo { .. }));
    }
}
