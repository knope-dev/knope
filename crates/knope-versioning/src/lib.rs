pub mod semver;

pub use semver::{Label, PreVersion, Prerelease, StableVersion, Version};

// #[derive(Debug)]
// pub enum VersionedFile {
//     Cargo(Cargo),
//     PubSpec(PubSpec),
//     GoMod(GoMod),
//     PackageJson(PackageJson),
//     PyProject(PyProject),
// }
//
// impl VersionedFile {
//     pub fn update_version(self, new_version: Version) -> Self {}
// }
