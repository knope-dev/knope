use serde::Deserialize;
use toml::Spanned;

pub(crate) fn get_version(content: &str) -> Result<String, toml::de::Error> {
    toml::from_str::<Cargo>(content).map(|cargo| cargo.package.version.into_inner())
}

pub(crate) fn set_version(
    mut cargo_toml: String,
    new_version: &str,
) -> Result<String, toml::de::Error> {
    let doc: Cargo = toml::from_str(&cargo_toml)?;

    // Account for quotes with +- 1
    let start = doc.package.version.start() + 1;
    let end = doc.package.version.end() - 1;

    cargo_toml.replace_range(start..end, new_version);

    Ok(cargo_toml)
}

#[derive(Debug, Deserialize)]
struct Cargo {
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Package {
    version: Spanned<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version() {
        let content = r###"
        [package]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        assert_eq!(get_version(content).unwrap(), "0.1.0-rc.0".to_string());
    }

    #[test]
    fn test_set_version() {
        let content = r###"
        [package]
        name = "tester"
        version = "0.1.0-rc.0"
        "###;

        let new = set_version(String::from(content), "1.2.3-rc.4").unwrap();

        let expected = content.replace("0.1.0-rc.0", "1.2.3-rc.4");
        assert_eq!(new, expected);
    }
}
