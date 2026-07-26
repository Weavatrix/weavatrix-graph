use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use weavatrix_graph::{EdgeKind, EvidenceKind, NodeKind};

#[test]
fn rust_sources_stay_small_and_focused() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    collect_rust_sources(&root, &mut sources);

    for path in sources {
        let lines = fs::read_to_string(&path).unwrap().lines().count();
        assert!(
            lines <= 300,
            "{} has {lines} lines; split files before they become monoliths",
            path.strip_prefix(&root).unwrap().display()
        );
    }
}

#[test]
fn domain_modules_use_facades_with_focused_leaf_files() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert_leaf_modules(
        &root,
        "src/model.rs",
        "src/model",
        &["element.rs", "id.rs", "provenance.rs", "span.rs"],
    );
    assert_leaf_modules(
        &root,
        "src/kind.rs",
        "src/kind",
        &["edge.rs", "evidence.rs", "node.rs", "string.rs"],
    );
    assert_leaf_modules(
        &root,
        "src/topology.rs",
        "src/topology",
        &["core.rs", "csr.rs", "index.rs", "parallel.rs", "view.rs"],
    );
    assert_leaf_modules(
        &root,
        "src/traversal_cache.rs",
        "src/traversal_cache",
        &[
            "adaptive.rs",
            "build.rs",
            "core.rs",
            "eager.rs",
            "elias_fano.rs",
            "ergonomics.rs",
            "iter.rs",
            "packed.rs",
            "parallel.rs",
            "walk.rs",
        ],
    );
    assert_leaf_modules(
        &root,
        "src/graph.rs",
        "src/graph",
        &[
            "builder.rs",
            "bulk.rs",
            "core.rs",
            "index.rs",
            "validate.rs",
        ],
    );
    assert_leaf_modules(
        &root,
        "src/working.rs",
        "src/working",
        &["core.rs", "freeze.rs", "key.rs", "mutate.rs", "view.rs"],
    );
    assert_leaf_modules(
        &root,
        "src/algo.rs",
        "src/algo",
        &[
            "astar.rs",
            "bellman.rs",
            "components.rs",
            "dominators.rs",
            "flow.rs",
            "mst.rs",
            "rank.rs",
            "shortest.rs",
            "transitive.rs",
            "traversal.rs",
        ],
    );
    assert_leaf_modules(
        &root,
        "src/algo/flow.rs",
        "src/algo/flow",
        &["core.rs", "cut.rs"],
    );
    assert_leaf_modules(
        &root,
        "src/algo/components.rs",
        "src/algo/components",
        &["condensation.rs", "dag.rs", "scc.rs", "weak.rs"],
    );
    assert_leaf_modules(
        &root,
        "src/undirected.rs",
        "src/undirected",
        &["core.rs", "view.rs"],
    );
    assert_leaf_modules(&root, "src/matrix.rs", "src/matrix", &["dense.rs"]);
    assert_leaf_modules(
        &root,
        "src/generator.rs",
        "src/generator",
        &["deterministic.rs", "random.rs"],
    );
}

#[test]
fn dependency_declarations_are_not_duplicated() {
    let manifest =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();

    for section in ["dependencies", "dev-dependencies"] {
        let names = dependency_names(&manifest, section);
        let unique = names.iter().collect::<BTreeSet<_>>();
        assert_eq!(
            names.len(),
            unique.len(),
            "[{section}] contains duplicate package declarations"
        );
    }
}

#[test]
fn canonical_kind_strings_are_unique_within_each_contract() {
    let node_kinds = [
        NodeKind::Repository,
        NodeKind::File,
        NodeKind::Module,
        NodeKind::Package,
        NodeKind::Function,
        NodeKind::Method,
        NodeKind::Struct,
        NodeKind::Enum,
        NodeKind::Trait,
        NodeKind::TypeAlias,
        NodeKind::Constant,
        NodeKind::Static,
        NodeKind::Service,
        NodeKind::Endpoint,
        NodeKind::Table,
        NodeKind::Column,
        NodeKind::Topic,
        NodeKind::ConsumerGroup,
        NodeKind::Exchange,
        NodeKind::Queue,
        NodeKind::Binding,
        NodeKind::Collection,
        NodeKind::Index,
        NodeKind::KubernetesResource,
        NodeKind::Container,
        NodeKind::ConfigKey,
        NodeKind::Unknown,
    ];
    let edge_kinds = [
        EdgeKind::Contains,
        EdgeKind::Imports,
        EdgeKind::Calls,
        EdgeKind::References,
        EdgeKind::Method,
        EdgeKind::Implements,
        EdgeKind::ReExports,
        EdgeKind::DependsOn,
        EdgeKind::Inherits,
        EdgeKind::Publishes,
        EdgeKind::Consumes,
        EdgeKind::Binds,
        EdgeKind::Reads,
        EdgeKind::Writes,
        EdgeKind::Deploys,
        EdgeKind::Exposes,
        EdgeKind::Mounts,
        EdgeKind::Configures,
    ];
    let evidence_kinds = [
        EvidenceKind::ExactLsp,
        EvidenceKind::Extracted,
        EvidenceKind::ResolvedCanonical,
        EvidenceKind::Inferred,
        EvidenceKind::Conflict,
        EvidenceKind::Parsed,
        EvidenceKind::Resolved,
        EvidenceKind::Manifest,
        EvidenceKind::Literal,
        EvidenceKind::Toolchain,
        EvidenceKind::Runtime,
    ];

    assert_unique("node", node_kinds.iter().map(NodeKind::as_str));
    assert_unique("edge", edge_kinds.iter().map(EdgeKind::as_str));
    assert_unique("evidence", evidence_kinds.iter().map(EvidenceKind::as_str));
}

fn assert_leaf_modules(root: &Path, facade: &str, directory: &str, leaves: &[&str]) {
    let facade_lines = fs::read_to_string(root.join(facade))
        .unwrap()
        .lines()
        .count();
    assert!(
        facade_lines <= 40,
        "{facade} should stay a small re-export facade, not hold domain logic"
    );
    for leaf in leaves {
        assert!(
            root.join(directory).join(leaf).is_file(),
            "{directory}/{leaf} is required by the module layout"
        );
    }
}

fn assert_unique<'value>(contract: &str, values: impl IntoIterator<Item = &'value str>) {
    let values = values.into_iter().collect::<Vec<_>>();
    let unique = values.iter().collect::<BTreeSet<_>>();
    assert_eq!(
        values.len(),
        unique.len(),
        "{contract} kind contract contains duplicate wire values"
    );
}

fn dependency_names(manifest: &str, section: &str) -> Vec<String> {
    manifest
        .split(&format!("[{section}]"))
        .nth(1)
        .unwrap_or_default()
        .split('[')
        .next()
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('=').map(|(name, _)| name.trim().to_owned()))
        .collect()
}

fn collect_rust_sources(directory: &Path, output: &mut Vec<PathBuf>) {
    if directory.ends_with("target") || directory.ends_with(".git") {
        return;
    }
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
