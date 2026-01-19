use std::collections::BTreeMap;

use super::{Label, Prerelease};

#[derive(Clone, Debug, Eq, PartialEq)]
/// Used to track the various pre-releases of a version, can never be empty
pub(crate) struct PrereleaseMap(BTreeMap<Label, Prerelease>);

impl PrereleaseMap {
    /// Create a new map, cannot be empty
    pub(crate) fn new(prerelease: Prerelease) -> Self {
        let mut map = BTreeMap::new();
        map.insert(prerelease.label.clone(), prerelease);
        Self(map)
    }

    #[allow(clippy::unwrap_used)] // Map is not allowed to be empty ever
    pub(crate) fn last(&self) -> &Prerelease {
        self.0
            .last_key_value()
            .map(|(_label, prerelease)| prerelease)
            .unwrap()
    }

    pub(crate) fn insert(&mut self, prerelease: Prerelease) {
        self.0.insert(prerelease.label.clone(), prerelease);
    }

    pub(crate) fn get(&self, key: &Label) -> Option<&Prerelease> {
        self.0.get(key)
    }
}
