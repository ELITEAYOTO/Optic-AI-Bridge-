use std::{fs, path::Path};

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("app crate must live under workspace/crates")
}

#[test]
fn project_version_has_one_canonical_value() {
    let root = workspace_root();
    let expected = fs::read_to_string(root.join("VERSION"))
        .expect("read VERSION")
        .trim()
        .to_owned();
    assert!(!expected.is_empty(), "VERSION must not be empty");
    assert_eq!(env!("CARGO_PKG_VERSION"), expected);

    let plugin: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("packaging/chatgpt-plugin/.codex-plugin/plugin.json"))
            .expect("read plugin manifest"),
    )
    .expect("parse plugin manifest");
    assert_eq!(
        plugin.get("version").and_then(serde_json::Value::as_str),
        Some(expected.as_str()),
        "ChatGPT plugin version must match VERSION"
    );

    let cargo = fs::read_to_string(root.join("Cargo.toml")).expect("read workspace Cargo.toml");
    let workspace_version = cargo
        .lines()
        .skip_while(|line| line.trim() != "[workspace.package]")
        .skip(1)
        .find_map(|line| {
            let line = line.trim();
            line.strip_prefix("version = ")
                .map(|value| value.trim_matches('"'))
        })
        .expect("workspace package version");
    assert_eq!(workspace_version, expected);
}

#[test]
fn every_optic_crate_inherits_workspace_version() {
    let crates = workspace_root().join("crates");
    for entry in fs::read_dir(crates).expect("read crates directory") {
        let entry = entry.expect("crate directory entry");
        if !entry.file_type().expect("crate file type").is_dir() {
            continue;
        }
        let manifest = entry.path().join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let contents = fs::read_to_string(&manifest).expect("read crate manifest");
        assert!(
            contents
                .lines()
                .any(|line| line.trim() == "version.workspace = true"),
            "{} must inherit the canonical workspace version",
            manifest.display()
        );
    }
}
