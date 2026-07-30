use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn production_modules_avoid_panic_shortcuts() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let forbidden = [
        ".unwrap()",
        ".expect(",
        "panic!(",
        "unreachable!(",
        "unimplemented!(",
        "todo!(",
    ];
    let mut violations = Vec::new();
    for file in rust_files(&source) {
        let text = fs::read_to_string(&file).expect("repository Rust source is readable");
        for (offset, line) in text.lines().enumerate() {
            if let Some(pattern) = forbidden.iter().find(|pattern| line.contains(**pattern)) {
                violations.push(format!(
                    "{}:{} contains {pattern}",
                    file.display(),
                    offset + 1
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "production panic shortcuts are forbidden:\n{}",
        violations.join("\n")
    );
}

fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut pending = vec![directory.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        let entries = fs::read_dir(&path).expect("repository source directory is readable");
        for entry in entries {
            let path = entry
                .expect("repository directory entry is readable")
                .path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}
