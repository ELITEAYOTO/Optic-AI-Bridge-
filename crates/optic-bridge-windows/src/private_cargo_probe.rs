//! Fixed public Cargo fixture only, not a production output collector.
use super::*;
use crate::open_file_for_delete_no_reparse;

const PACKAGE: &str = "optic_private_cargo_probe";
const MANIFEST: &str = "[package]\nname=\"optic_private_cargo_probe\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[profile.dev]\ndebug=0\nincremental=false\n[profile.test]\ndebug=0\nincremental=false\n";
const LOCK: &str =
    "version = 4\n\n[[package]]\nname = \"optic_private_cargo_probe\"\nversion = \"0.0.0\"\n";
const SOURCE: &str = "pub fn add(a:i32,b:i32)->i32 { a+b }\n#[cfg(test)] mod tests { #[test] fn actual_sum() { assert_eq!(super::add(2,3),5); } }\n";

#[test]
fn private_cargo_null_device_child() {
    if std::env::var("OPTIC_CARGO_NULL_PROBE").as_deref() != Ok("1") {
        return;
    }
    for name in ["NUL", r"\\.\NUL"] {
        let read = OpenOptions::new().read(true).open(name);
        let write = OpenOptions::new().write(true).open(name);
        println!(
            "PRIVATE_CARGO_NULL_DEVICE: path={name:?} read_error={:?} write_error={:?}",
            read.as_ref().err().and_then(io::Error::raw_os_error),
            write.as_ref().err().and_then(io::Error::raw_os_error)
        );
    }
}

pub(super) fn fixed_environment(root: &Path, env: &mut Vec<(OsString, OsString)>) {
    let mut flags = OsString::from("--sysroot\x1f");
    flags.push(root);
    flags.push("\x1f-Clinker=");
    flags.push(root.join("bin/native-linker/link.exe"));
    flags.push("\x1f-Clink-arg=/manifest:no\x1f-Clink-arg=/incremental:no");
    env.extend([
        ("RUSTC".into(), root.join("bin/rustc.exe").into_os_string()),
        (
            "CARGO_HOME".into(),
            root.join("cargo-home").into_os_string(),
        ),
        ("CARGO_ENCODED_RUSTFLAGS".into(), flags),
        ("CARGO_INCREMENTAL".into(), "0".into()),
        // Only the copied application CRT DLLs, never ambient user PATH.
        ("PATH".into(), root.join("output").into_os_string()),
    ]);
}

fn invoke(owner: &OwnedProbeProfile, root: &Path, test: bool) -> Compilation {
    let mut args: Vec<OsString> = vec![
        if test { "test" } else { "check" }.into(),
        "--offline".into(),
        "--frozen".into(),
        "--lib".into(),
        "--jobs=1".into(),
        "--target".into(),
        TARGET.into(),
        "--manifest-path".into(),
        root.join("cargo-project/Cargo.toml").into_os_string(),
        "--target-dir".into(),
        root.join("cargo-target").into_os_string(),
    ];
    if test {
        args.extend(["--".into(), "--test-threads=1".into()]);
    }
    run_native_with_process_limit(owner, root, &root.join("bin/cargo.exe"), &args, 3).unwrap()
}

fn require_success(value: &Compilation) {
    assert_eq!(
        value.exit,
        0,
        "Cargo stdout={} stderr={}",
        String::from_utf8_lossy(&value.stdout),
        String::from_utf8_lossy(&value.stderr)
    );
}

fn observe_null_device(owner: &OwnedProbeProfile, root: &Path, remaining: &mut u64) -> Vec<String> {
    // Compare fixed null-device opens without changing device ACLs.
    let control = File::open(r"\\.\NUL");
    println!(
        "PRIVATE_CARGO_NULL_PARENT: read_error={:?}",
        control.as_ref().err().and_then(io::Error::raw_os_error)
    );
    drop(control);
    let null_probe = root.join("output/cargo-null-probe.exe");
    copy_exact(&std::env::current_exe().unwrap(), &null_probe, remaining).unwrap();
    let null_result = run_native(
        owner,
        root,
        &null_probe,
        &[
            "--exact".into(),
            "rust_compile_probe::cargo_probe::private_cargo_null_device_child".into(),
            "--nocapture".into(),
            "--test-threads=1".into(),
        ],
    )
    .unwrap();
    require_success(&null_result);
    println!("{}", String::from_utf8_lossy(&null_result.stdout));
    assert!(String::from_utf8_lossy(&null_result.stdout).contains("PRIVATE_CARGO_NULL_DEVICE:"));
    fs::remove_file(&null_probe).unwrap();
    let observations: Vec<_> = String::from_utf8_lossy(&null_result.stdout)
        .lines()
        .filter_map(|line| {
            line.find("PRIVATE_CARGO_NULL_DEVICE:")
                .map(|offset| line[offset..].to_owned())
        })
        .collect();
    assert_eq!(observations.len(), 2);
    observations
}

#[test]
fn private_null_device_comparison_without_toolchain() {
    let mut owner = OwnedProbeProfile {
        profile: Some(
            crate::EphemeralAppContainerPlan::generate()
                .unwrap()
                .create()
                .unwrap(),
        ),
        uncertain: Cell::new(false),
    };
    let root = owner
        .profile
        .as_ref()
        .unwrap()
        .private_storage_root()
        .unwrap()
        .join("null-device-probe");
    fs::create_dir(&root).unwrap();
    for name in ["tmp", "local", "roaming", "output"] {
        fs::create_dir(root.join(name)).unwrap();
    }
    let mut remaining = MAX_RUNTIME;
    observe_null_device(&owner, &root, &mut remaining);
    assert!(!owner.uncertain.get());
    for name in ["output", "roaming", "local", "tmp"] {
        fs::remove_dir(root.join(name)).unwrap();
    }
    fs::remove_dir(&root).unwrap();
    owner.profile.take().unwrap().close().unwrap();
    println!(
        "PRIVATE_NULL_COMPARISON_COMPLETE: observed only, no runtime selected, device ACL unchanged, owned profile closed"
    );
}

pub(super) fn run(
    owner: &OwnedProbeProfile,
    root: &Path,
    compiler_bin: &Path,
    remaining: &mut u64,
) {
    let before_null = observe_null_device(owner, root, remaining);
    let null_grant = if std::env::var("OPTIC_DISPOSABLE_NULL_DEVICE_COMPARISON").as_deref()
        == Ok("ephemeral-profile-only")
    {
        let grant = owner
            .profile
            .as_ref()
            .unwrap()
            .grant_null_stdio_disposable_probe()
            .unwrap();
        let prepared = observe_null_device(owner, root, remaining);
        assert!(
            prepared
                .iter()
                .all(|line| line.contains("read_error=None write_error=None")),
            "exact profile NUL grant did not enable stdio: {prepared:?}"
        );
        println!(
            "PRIVATE_CARGO_NULL_PREPARED: exact ephemeral profile SID only; hosted-CI comparison, no production fallback"
        );
        Some(grant)
    } else {
        None
    };
    let selected = fs::canonicalize(PathBuf::from(
        std::env::var_os("OPTIC_PRIVATE_CARGO").expect("explicit direct Cargo selection"),
    ))
    .unwrap();
    assert_eq!(
        selected,
        fs::canonicalize(compiler_bin.join("cargo.exe")).unwrap(),
        "Cargo must come from the exact selected Rust toolchain"
    );
    let cargo = root.join("bin/cargo.exe");
    copy_exact(&selected, &cargo, remaining).unwrap();
    let project = root.join("cargo-project");
    for path in [
        &project,
        &project.join("src"),
        &root.join("cargo-home"),
        &root.join("cargo-target"),
    ] {
        fs::create_dir(path).unwrap();
    }
    let manifest = project.join("Cargo.toml");
    let lock = project.join("Cargo.lock");
    let source = project.join("src/lib.rs");
    fs::write(&manifest, MANIFEST).unwrap();
    fs::write(&lock, LOCK).unwrap();
    fs::write(&source, SOURCE).unwrap();
    require_success(&invoke(owner, root, false));
    fs::write(&source, SOURCE.replace("a+b", "a-b")).unwrap();
    let failed = invoke(owner, root, true);
    assert_ne!(failed.exit, 0, "actual broken assertion must fail");
    assert!(
        String::from_utf8_lossy(&failed.stdout).contains("assertion"),
        "missing real failing assertion: {} / {}",
        String::from_utf8_lossy(&failed.stdout),
        String::from_utf8_lossy(&failed.stderr)
    );
    fs::write(&source, SOURCE).unwrap();
    let passed = invoke(owner, root, true);
    require_success(&passed);
    assert!(String::from_utf8_lossy(&passed.stdout).contains("test result: ok. 1 passed"));
    fs::write(
        &manifest,
        format!("{MANIFEST}[dependencies]\noptic_deliberately_unavailable_dependency=\"=0.0.0\"\n"),
    )
    .unwrap();
    let unavailable = invoke(owner, root, false);
    assert_ne!(unavailable.exit, 0);
    assert!(
        String::from_utf8_lossy(&unavailable.stderr).contains("no matching package"),
        "offline failure must be dependency absence: {}",
        String::from_utf8_lossy(&unavailable.stderr)
    );
    fs::write(&manifest, MANIFEST).unwrap();
    require_success(&invoke(owner, root, false));
    assert_eq!(fs::read(&source).unwrap(), SOURCE.as_bytes());
    assert_eq!(fs::read(&manifest).unwrap(), MANIFEST.as_bytes());
    assert_eq!(fs::read(&lock).unwrap(), LOCK.as_bytes());
    assert!(!owner.uncertain.get());
    clean_fixed_tree(&root.join("cargo-target"), false).unwrap();
    clean_fixed_tree(&root.join("cargo-home"), true).unwrap();
    for file in [&source, &lock, &manifest, &cargo] {
        fs::remove_file(file).unwrap();
    }
    fs::remove_dir(project.join("src")).unwrap();
    fs::remove_dir(project).unwrap();
    if let Some(grant) = null_grant {
        grant.revoke().unwrap();
        assert_eq!(
            observe_null_device(owner, root, remaining),
            before_null,
            "explicit NUL revoke must restore observed profile access"
        );
        println!(
            "PRIVATE_CARGO_NULL_REVOKED: explicit revoke acknowledged and original profile access restored"
        );
    }
    println!(
        "PRIVATE_CARGO_EXECUTION_OBSERVED: actual offline check/test, assertion failure and corrected test; missing dependency refused; known outputs removed after owned-tree quiescence"
    );
}

fn hash_name(value: &str, prefix: &str, suffix: &str) -> bool {
    value
        .strip_prefix(prefix)
        .and_then(|s| s.strip_suffix(suffix))
        .is_some_and(|h| h.len() == 16 && h.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn known_name(name: &str, directory: bool, home: bool) -> bool {
    if home {
        return !directory
            && matches!(
                name,
                ".package-cache" | ".package-cache-mutate" | ".global-cache"
            );
    }
    if directory {
        return matches!(
            name,
            TARGET | "debug" | "deps" | ".fingerprint" | "build" | "examples" | "incremental"
        ) || hash_name(name, &format!("{PACKAGE}-"), "");
    }
    matches!(
        name,
        "CACHEDIR.TAG"
            | ".rustc_info.json"
            | ".cargo-lock"
            | ".cargo-artifact-lock"
            | "invoked.timestamp"
            | "dep-lib-optic_private_cargo_probe"
            | "lib-optic_private_cargo_probe"
            | "lib-optic_private_cargo_probe.json"
            | "dep-test-lib-optic_private_cargo_probe"
            | "test-lib-optic_private_cargo_probe"
            | "test-lib-optic_private_cargo_probe.json"
    ) || [".d", ".exe", ".pdb"]
        .iter()
        .any(|suffix| hash_name(name, &format!("{PACKAGE}-"), suffix))
        || [".rlib", ".rmeta"]
            .iter()
            .any(|suffix| hash_name(name, &format!("lib{PACKAGE}-"), suffix))
}

fn clean_fixed_tree(root: &Path, home: bool) -> io::Result<()> {
    let canonical = fs::canonicalize(root)?;
    let root = canonical.as_path();
    let mut pending = vec![(root.to_path_buf(), 0_usize)];
    let mut directories = Vec::new();
    let mut files = Vec::new();
    let mut pins = Vec::new();
    let mut bytes = 0_u64;
    let mut entries = 1_usize;
    let mut unknown = Vec::new();
    // Validate the whole bounded known-name inventory before deleting any file.
    while let Some((directory, depth)) = pending.pop() {
        if depth > 8 || directories.len() + files.len() >= 256 {
            return Err(io::Error::other("fixed Cargo inventory bound"));
        }
        let relative = directory.strip_prefix(root).unwrap();
        let pin = pin_directory_chain(root, Some(relative)).map_err(io::Error::other)?;
        for entry in fs::read_dir(&directory)? {
            if entries == 256 {
                return Err(io::Error::other("fixed Cargo inventory bound"));
            }
            entries += 1;
            let entry = entry?;
            let path = entry.path();
            let meta = fs::symlink_metadata(&path)?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| io::Error::other("Cargo non-Unicode name"))?;
            if !known_name(&name, meta.is_dir(), home) {
                unknown.push(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .chars()
                        .take(120)
                        .collect::<String>(),
                );
                if unknown.len() == 8 {
                    return Err(io::Error::other(format!(
                        "unrecorded fixed Cargo products: home={home} first8={unknown:?}"
                    )));
                }
                // Unknown directories are never traversed and unknown file
                // contents are not opened. No deletion occurs on this path.
                continue;
            }
            if meta.is_dir() {
                pending.push((path, depth + 1));
            } else {
                let mut opened = open_exclusive_read_file(&path).map_err(io::Error::other)?;
                let size = opened.file_mut().metadata()?.len();
                bytes = bytes
                    .checked_add(size)
                    .ok_or_else(|| io::Error::other("Cargo byte overflow"))?;
                if opened.final_path() != path
                    || size > 16 * 1024 * 1024
                    || bytes > 64 * 1024 * 1024
                {
                    return Err(io::Error::other("Cargo collected file bound"));
                }
                files.push((path, opened.identity()));
            }
        }
        pins.push(pin);
        directories.push(directory);
    }
    if !unknown.is_empty() {
        return Err(io::Error::other(format!(
            "unrecorded fixed Cargo products: home={home} names={unknown:?}"
        )));
    }
    for (path, identity) in &files {
        let file = open_file_for_delete_no_reparse(path).map_err(io::Error::other)?;
        if file.identity() != *identity || file.final_path() != path {
            return Err(io::Error::other("Cargo product changed before cleanup"));
        }
        file.delete().map_err(io::Error::other)?;
    }
    drop(pins);
    for directory in directories.into_iter().rev() {
        fs::remove_dir(directory)?;
    }
    println!(
        "PRIVATE_CARGO_KNOWN_OUTPUTS: home={home} files={} bytes={bytes}; post-exit collection bounds only",
        files.len()
    );
    Ok(())
}

#[test]
fn fixed_cargo_collection_rejects_unknown_aliases_and_oversize_before_deletion() {
    let root = std::env::temp_dir().join(format!(
        "optic-fixed-cargo-inventory-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let known = root.join("CACHEDIR.TAG");
    let unknown = root.join("unrecorded.txt");
    fs::write(&known, b"fixed").unwrap();
    fs::write(&unknown, b"retain").unwrap();
    assert!(clean_fixed_tree(&root, false).is_err());
    assert_eq!(fs::read(&known).unwrap(), b"fixed");
    assert_eq!(fs::read(&unknown).unwrap(), b"retain");
    fs::remove_file(&unknown).unwrap();
    let alias = root.join(".rustc_info.json");
    fs::hard_link(&known, &alias).unwrap();
    assert!(clean_fixed_tree(&root, false).is_err());
    assert!(known.exists() && alias.exists());
    fs::remove_file(&alias).unwrap();
    let writer = OpenOptions::new().write(true).open(&known).unwrap();
    assert!(clean_fixed_tree(&root, false).is_err());
    drop(writer);
    if std::os::windows::fs::symlink_file(&known, &alias).is_ok() {
        assert!(clean_fixed_tree(&root, false).is_err());
        assert!(known.exists() && alias.exists());
        fs::remove_file(&alias).unwrap();
    }
    OpenOptions::new()
        .write(true)
        .open(&known)
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();
    assert!(clean_fixed_tree(&root, false).is_err());
    assert!(known.exists());
    fs::write(&known, b"fixed").unwrap();
    let artifact_lock = root.join(".cargo-artifact-lock");
    fs::write(&artifact_lock, b"").unwrap();
    clean_fixed_tree(&root, false).unwrap();
    assert!(!root.exists());
}
