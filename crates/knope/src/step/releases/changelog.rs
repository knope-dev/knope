use knope_versioning::release_notes::Changelog;
use relative_path::RelativePathBuf;

use crate::fs;

pub(crate) fn load_changelog(path: RelativePathBuf) -> Result<Changelog, fs::Error> {
    let path_buf = path.to_path("");
    let content = if path_buf.exists() {
        fs::read_to_string(path_buf)?
    } else {
        String::new()
    };

    Ok(Changelog::new(path, content))
}
