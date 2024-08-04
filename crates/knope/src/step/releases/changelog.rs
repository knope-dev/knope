use knope_versioning::release_notes::{Changelog, HeaderLevel};
use relative_path::RelativePathBuf;

use crate::fs;

pub(crate) fn load_changelog(path: RelativePathBuf) -> Result<Changelog, fs::Error> {
    let path_buf = path.to_path("");
    let content = if path_buf.exists() {
        fs::read_to_string(path_buf)?
    } else {
        String::new()
    };
    let release_header_level = content
        .lines()
        .filter(|line| line.starts_with('#'))
        .nth(1)
        .and_then(|header| {
            if header.starts_with("##") {
                Some(HeaderLevel::H2)
            } else if header.starts_with('#') {
                Some(HeaderLevel::H1)
            } else {
                None
            }
        })
        .unwrap_or(HeaderLevel::H2);
    Ok(Changelog {
        path,
        content,
        release_header_level,
    })
}
