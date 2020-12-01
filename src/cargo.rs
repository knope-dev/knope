use color_eyre::Result;
use serde::Deserialize;
use toml_edit::{value, Document};

pub(crate) fn get_version() -> Option<String> {
    Some(
        toml::from_str::<Cargo>(&std::fs::read_to_string("Cargo.toml").ok()?)
            .ok()?
            .package
            .version,
    )
}

pub(crate) fn set_version(new_version: &str) -> Result<()> {
    let toml = std::fs::read_to_string("Cargo.toml")?;
    let mut doc = toml.parse::<Document>()?;
    doc["package"]["version"] = value(new_version);
    std::fs::write("Cargo.toml", doc.to_string())?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Cargo {
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Package {
    version: String,
}
