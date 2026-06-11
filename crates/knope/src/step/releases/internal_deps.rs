use std::collections::{HashMap, HashSet, VecDeque};

use knope_config::InternalDependencyUpdate;
use knope_versioning::{
    changes::{Change, ChangeSource, ChangeType},
    package::Name,
    semver::Version,
};
use relative_path::RelativePathBuf;
use tracing::debug;

use super::Package;

/// A bumped dependency that should be reflected in a dependent's release notes.
#[derive(Clone, Debug)]
pub(super) struct BumpedDependency {
    pub name: Name,
    pub new_version: Version,
}

/// Build dependents-of edges: `dependents[D]` = indices of packages that depend on D.
///
/// A package P "owns" a path if it has a `versioned_files` entry for that path with no
/// `dependency` set (i.e., the file defines P's own version). A `{ path, dependency }` entry
/// in package D's `versioned_files` means D's version gets written into that file, so when
/// the file is owned by some other package O, that implies the edge "O depends on D"
/// (regardless of the name D goes by inside the file). Lock files record versions rather
/// than declare dependencies, so they neither own nor imply edges. Packages can also declare
/// dependencies explicitly via `internal_dependencies` for relationships that aren't visible
/// in `versioned_files` (e.g., versions tracked only in a shared workspace manifest).
///
/// Edges to dependents that opted out (`update_internal_dependencies = "none"`) are skipped
/// entirely: they can't receive propagation, and including them would only change the order
/// in which packages are processed.
pub(super) fn build_dependents(packages: &[Package]) -> HashMap<Name, Vec<usize>> {
    let mut owners: HashMap<RelativePathBuf, usize> = HashMap::new();
    for (idx, pkg) in packages.iter().enumerate() {
        for vf in pkg.versioning.versioned_files() {
            if vf.dependency.is_none() && !vf.is_lock_file() {
                owners.insert(vf.as_path(), idx);
            }
        }
    }
    let name_to_idx: HashMap<&str, usize> = packages
        .iter()
        .enumerate()
        .map(|(idx, pkg)| (pkg.name().as_ref(), idx))
        .collect();

    let mut edges: HashMap<Name, Vec<usize>> = HashMap::new();
    // (dependent, dependency) pairs already recorded
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut add_edge = |dependency_idx: usize, dependent_idx: usize, seen: &mut HashSet<_>| {
        let Some(dependency) = packages.get(dependency_idx) else {
            return;
        };
        let Some(dependent) = packages.get(dependent_idx) else {
            return;
        };
        if dependent_idx == dependency_idx
            || dependent.update_internal_dependencies == InternalDependencyUpdate::None
            || !seen.insert((dependent_idx, dependency_idx))
        {
            return;
        }
        edges
            .entry(dependency.name().clone())
            .or_default()
            .push(dependent_idx);
    };

    for (dependency_idx, pkg) in packages.iter().enumerate() {
        for vf in pkg.versioning.versioned_files() {
            if vf.dependency.is_none() || vf.is_lock_file() {
                continue;
            }
            let Some(&dependent_idx) = owners.get(&vf.as_path()) else {
                continue;
            };
            add_edge(dependency_idx, dependent_idx, &mut seen);
        }
    }

    for (dependent_idx, pkg) in packages.iter().enumerate() {
        for dep_name in &pkg.internal_dependencies {
            let Some(&dependency_idx) = name_to_idx.get(dep_name.as_str()) else {
                debug!(
                    "internal_dependencies of {pkg} names unknown package {dep_name}; ignoring",
                    pkg = pkg.name()
                );
                continue;
            };
            add_edge(dependency_idx, dependent_idx, &mut seen);
        }
    }
    edges
}

/// Topologically sort package indices so that a package appears after all of its internal
/// dependencies. Falls back to original order on cycles (we still return every index).
#[expect(
    clippy::indexing_slicing,
    reason = "all indices come from 0..packages.len() or from `dependents` values that were built from those same indices"
)]
pub(super) fn topological_order(
    packages: &[Package],
    dependents: &HashMap<Name, Vec<usize>>,
) -> Vec<usize> {
    let n = packages.len();
    let mut in_degree = vec![0usize; n];
    for targets in dependents.values() {
        for &t in targets {
            if t < n {
                in_degree[t] += 1;
            }
        }
    }
    // FIFO so that packages with no dependency relationship keep their config order.
    let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut visited = vec![false; n];
    let mut order = Vec::with_capacity(n);
    while let Some(idx) = queue.pop_front() {
        if visited[idx] {
            continue;
        }
        visited[idx] = true;
        order.push(idx);
        if let Some(targets) = dependents.get(packages[idx].name()) {
            for &t in targets {
                if t >= n || visited[t] {
                    continue;
                }
                in_degree[t] = in_degree[t].saturating_sub(1);
                if in_degree[t] == 0 {
                    queue.push_back(t);
                }
            }
        }
    }
    // Append any remaining (would happen if there's a cycle) — better to release them than panic.
    for idx in 0..n {
        if !visited[idx] {
            debug!(
                "Internal dependency cycle detected; falling back to source order for {pkg}",
                pkg = packages[idx].name()
            );
            order.push(idx);
        }
    }
    order
}

/// Build a synthetic [`Change`] representing "this package was bumped because its internal
/// dependencies updated." The change is grouped with any others on the same package, and
/// rendered as an "Updated dependencies" section in the release notes.
pub(super) fn synthetic_change(
    policy: InternalDependencyUpdate,
    bumps: &[BumpedDependency],
) -> Option<Change> {
    match policy {
        InternalDependencyUpdate::None => None,
        InternalDependencyUpdate::Patch | InternalDependencyUpdate::Minor => {
            let change_type = match policy {
                InternalDependencyUpdate::Minor => ChangeType::Feature,
                _ => ChangeType::Fix,
            };
            let details = bumps
                .iter()
                .map(|bump| {
                    format!(
                        "  - {name}@{version}",
                        name = bump.name,
                        version = bump.new_version
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            Some(Change {
                change_type,
                summary: "Updated dependencies".to_string(),
                details: Some(details),
                original_source: ChangeSource::DependencyUpdate,
                git: None,
            })
        }
    }
}
