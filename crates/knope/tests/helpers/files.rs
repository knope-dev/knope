use std::path::Path;

pub fn copy_dir_contents(source: &Path, target: &Path) {
    for entry in source.read_dir().expect("read_dir call failed") {
        let entry = entry.expect("DirEntry");
        let path = entry.path();
        let target = target.join(path.file_name().unwrap());
        if path.is_dir() {
            std::fs::create_dir_all(&target).expect("create_dir_all call failed");
            copy_dir_contents(&path, &target);
        } else {
            std::fs::copy(&path, &target).expect("copy call failed");
        }
    }
}
