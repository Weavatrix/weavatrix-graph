use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn library_has_no_process_network_or_unscoped_unsafe_path() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    collect_rust_sources(&root.join("src"), &mut sources);
    let banned = [
        "std::process",
        "std::net",
        "std::fs",
        "reqwest",
        "tokio",
        "tree_sitter",
        "petgraph",
    ];
    for path in sources {
        let source = fs::read_to_string(&path).unwrap();
        for marker in banned {
            assert!(
                !source.contains(marker),
                "{} contains forbidden graph marker {marker}",
                path.display()
            );
        }
        let is_unsafe_fast = ["matrix/bit/unsafe_fast.rs", "topology/csr/unsafe_fast.rs"]
            .iter()
            .any(|allowed| path.ends_with(Path::new(allowed)));
        if !is_unsafe_fast {
            assert!(
                !source.contains("unsafe {") && !source.contains("unsafe fn"),
                "{} contains unsafe code outside an opt-in fast module",
                path.display()
            );
        }
    }
}

#[test]
fn runtime_dependencies_are_serde_plus_optional_rayon() {
    let manifest =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap()
        .split("[dev-dependencies]")
        .next()
        .unwrap();
    let entries = dependencies
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|line| {
        line.starts_with("serde =")
            && line.contains("default-features = false")
            && line.contains("\"alloc\"")
    }));
    assert!(
        entries
            .iter()
            .any(|line| line.starts_with("rayon =") && line.contains("optional = true"))
    );
    assert!(manifest.contains("unsafe-fast = []"));
}

fn collect_rust_sources(directory: &Path, output: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(directory)
        .unwrap()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}
