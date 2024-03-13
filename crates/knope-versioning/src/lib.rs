pub mod cargo;
mod pyproject;
pub mod semver;

pub use cargo::Cargo;
pub use pyproject::PyProject;
pub use semver::{Label, PreVersion, Prerelease, StableVersion, Version};

#[derive(Debug)]
pub enum VersionedFile {
    Cargo(Cargo),
    // PubSpec(PubSpec),
    // GoMod(GoMod),
    // PackageJson(PackageJson),
    PyProject(PyProject),
}

impl VersionedFile {
    #[must_use]
    pub fn set_version(self, new_version: Version) -> Self {
        match self {
            VersionedFile::Cargo(cargo) => VersionedFile::Cargo(cargo.set_version(new_version)),
            VersionedFile::PyProject(pyproject) => {
                VersionedFile::PyProject(pyproject.set_version(new_version))
            } // VersionedFile::PubSpec(pubspec) => {
              //     VersionedFile::PubSpec(pubspec.update_version(new_version))
              // }
              // VersionedFile::GoMod(gomod) => VersionedFile::GoMod(gomod.update_version(new_version)),
              // VersionedFile::PackageJson(package_json) => {
              //     VersionedFile::PackageJson(package_json.update_version(new_version))
              // }
        }
    }
}
