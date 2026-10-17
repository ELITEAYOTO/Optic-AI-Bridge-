//! Opt-in fixed public Rust compilation proof, never an execution/tool grant.
use crate::{
    AppContainerProfile, AppContainerStdio, CapturedOutputPipe, ContainedAppContainerProcess,
    LimitedJobObject, inspect_directory_no_reparse, open_file_no_reparse,
    spawn_appcontainer_with_environment_suspended,
};
use std::{
    cell::Cell,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::windows::io::AsHandle,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const TARGET: &str = "x86_64-pc-windows-msvc";
const MAX_RUNTIME: u64 = 512 * 1024 * 1024;
const MAX_FILE: u64 = 256 * 1024 * 1024;
const MAX_OUTPUT: usize = 128 * 1024;

fn copy_exact(source: &Path, destination: &Path, remaining: &mut u64) -> io::Result<()> {
    let mut source = open_file_no_reparse(source).map_err(io::Error::other)?;
    let size = source.file_mut().metadata()?.len();
    if size > MAX_FILE || size > *remaining {
        return Err(io::Error::other("fixed Rust runtime copy exceeds budget"));
    }
    let mut destination = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)?;
    let copied = io::copy(&mut source.file_mut().take(size), &mut destination)?;
    if copied != size || source.file_mut().read(&mut [0_u8; 1])? != 0 {
        return Err(io::Error::other("Rust runtime changed during copy"));
    }
    destination.flush()?;
    *remaining -= size;
    Ok(())
}

fn selected_files(directory: &Path, select: impl Fn(&str) -> bool) -> io::Result<Vec<PathBuf>> {
    inspect_directory_no_reparse(directory).map_err(io::Error::other)?;
    let mut selected = Vec::new();
    // Bound metadata discovery as well as the selected copy set.
    for (index, item) in fs::read_dir(directory)?.enumerate() {
        if index == 256 {
            return Err(io::Error::other("Rust runtime inventory exceeds bound"));
        }
        let item = item?;
        let name = item
            .file_name()
            .into_string()
            .map_err(|_| io::Error::other("non-Unicode runtime name"))?;
        if select(&name) {
            if selected.len() == 8 {
                return Err(io::Error::other("too many selected Rust runtime files"));
            }
            selected.push(item.path());
        }
    }
    selected.sort();
    Ok(selected)
}

struct Compilation {
    exit: u32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct OwnedProbeProfile {
    profile: Option<AppContainerProfile>,
    uncertain: Cell<bool>,
}
impl Drop for OwnedProbeProfile {
    fn drop(&mut self) {
        if self.uncertain.get() {
            // Retain only this fresh probe profile on uncertain native cleanup.
            // A kill request is not acknowledgement that storage can be deleted.
            std::mem::forget(self.profile.take());
        }
    }
}

fn compile(owner: &OwnedProbeProfile, root: &Path, source: &Path) -> io::Result<Compilation> {
    let executable = root.join("bin/rustc.exe");
    let args = [
        OsString::from("--crate-name"),
        "optic_public_probe".into(),
        "--crate-type=lib".into(),
        "--edition=2024".into(),
        "--target".into(),
        TARGET.into(),
        "--sysroot".into(),
        root.into(),
        "--emit=llvm-ir".into(),
        "-Cdebuginfo=0".into(),
        "-Cpanic=abort".into(),
        "-o".into(),
        "-".into(),
        source.into(),
    ];
    let input = File::open("NUL")?;
    let mut stdout = CapturedOutputPipe::new()?;
    let mut stderr = CapturedOutputPipe::new()?;
    let environment = vec![
        (
            "SystemRoot".into(),
            std::env::var_os("SystemRoot").ok_or_else(|| io::Error::other("SystemRoot"))?,
        ),
        ("TEMP".into(), root.join("tmp").into_os_string()),
        ("TMP".into(), root.join("tmp").into_os_string()),
        ("LOCALAPPDATA".into(), root.join("local").into_os_string()),
        ("APPDATA".into(), root.join("roaming").into_os_string()),
    ];
    owner.uncertain.set(true);
    let child = spawn_appcontainer_with_environment_suspended(
        owner.profile.as_ref().unwrap(),
        &executable,
        &args,
        root,
        AppContainerStdio {
            stdin: input.as_handle(),
            stdout: stdout.writer_handle()?,
            stderr: stderr.writer_handle()?,
        },
        &environment,
    )
    .map_err(|error| io::Error::other(format!("Rust suspended spawn failed: {error}")))?;
    let mut process = ContainedAppContainerProcess::contain(
        child,
        LimitedJobObject::new(1, 1024 * 1024 * 1024, 50)?,
    )?;
    stdout.close_parent_writer();
    stderr.close_parent_writer();
    let result = (|| {
        process.arm_deadline(15_000)?;
        process.resume()?;
        let deadline = Instant::now() + Duration::from_secs(17);
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let (mut out_eof, mut err_eof) = (false, false);
        loop {
            if !out_eof {
                let chunk = stdout.try_read(8192)?;
                out.extend(chunk.bytes);
                out_eof = chunk.eof;
            }
            if !err_eof {
                let chunk = stderr.try_read(8192)?;
                err.extend(chunk.bytes);
                err_eof = chunk.eof;
            }
            if out.len() + err.len() > MAX_OUTPUT {
                return Err(io::Error::other("Rust diagnostic output budget"));
            }
            if let Some(exit) = process.try_wait()?
                && out_eof
                && err_eof
            {
                return Ok(Compilation {
                    exit,
                    stdout: out,
                    stderr: err,
                });
            }
            if Instant::now() >= deadline {
                return Err(io::Error::other("Rust compile probe deadline"));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    })();
    // Error/overflow is not cleanup acknowledgement. Confirm the owned tree on
    // every monitor result before allowing private profile deletion.
    process.terminate_and_confirm_empty(2000)?;
    owner.uncertain.set(false);
    result
}

#[test]
#[ignore = "requires an explicitly selected installed direct Rust compiler"]
fn private_rust_compiles_fixed_library_and_reports_real_type_error() {
    let compiler = PathBuf::from(
        std::env::var_os("OPTIC_PRIVATE_RUSTC").expect("select installed direct rustc.exe"),
    );
    let compiler = fs::canonicalize(compiler).unwrap();
    assert_eq!(compiler.file_name().unwrap(), "rustc.exe");
    let bin = compiler.parent().unwrap();
    assert_eq!(bin.file_name().unwrap(), "bin");
    let sysroot = bin.parent().unwrap();
    let dlls = selected_files(bin, |name| name.to_ascii_lowercase().ends_with(".dll")).unwrap();
    assert!(
        !dlls.is_empty(),
        "direct compiler must have its pinned runtime DLL"
    );
    let lib_relative = PathBuf::from(format!("lib/rustlib/{TARGET}/lib"));
    let libraries = selected_files(&sysroot.join(&lib_relative), |name| {
        (name.starts_with("libcore-") || name.starts_with("libcompiler_builtins-"))
            && name.ends_with(".rlib")
    })
    .unwrap();
    assert_eq!(
        libraries.len(),
        2,
        "one core and compiler_builtins rlib from selected sysroot"
    );
    let mut owner = OwnedProbeProfile {
        profile: Some(AppContainerProfile::create_ephemeral().unwrap()),
        uncertain: Cell::new(false),
    };
    let root = owner
        .profile
        .as_ref()
        .unwrap()
        .private_storage_root()
        .unwrap()
        .join("rust-ir-probe");
    fs::create_dir(&root).unwrap();
    for relative in [
        "bin",
        "lib",
        "lib/rustlib",
        "lib/rustlib/x86_64-pc-windows-msvc",
        "lib/rustlib/x86_64-pc-windows-msvc/lib",
        "tmp",
        "local",
        "roaming",
    ] {
        fs::create_dir(root.join(relative)).unwrap();
    }
    let mut remaining = MAX_RUNTIME;
    copy_exact(&compiler, &root.join("bin/rustc.exe"), &mut remaining).unwrap();
    for dll in &dlls {
        copy_exact(
            dll,
            &root.join("bin").join(dll.file_name().unwrap()),
            &mut remaining,
        )
        .unwrap();
    }
    for library in &libraries {
        copy_exact(
            library,
            &root.join(&lib_relative).join(library.file_name().unwrap()),
            &mut remaining,
        )
        .unwrap();
    }
    let source = root.join("library.rs");
    let valid = b"#![no_std]\npub fn optic_add(a:i32,b:i32)->i32 { a+b }\n";
    fs::write(&source, valid).unwrap();
    let result = compile(&owner, &root, &source).unwrap();
    assert_eq!(
        result.exit,
        0,
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("optic_add"));
    assert_eq!(fs::read(&source).unwrap(), valid);
    let invalid = b"#![no_std]\npub fn optic_add()->i32 { true }\n";
    fs::write(&source, invalid).unwrap();
    let failure = compile(&owner, &root, &source).unwrap();
    assert_ne!(failure.exit, 0);
    assert!(
        String::from_utf8_lossy(&failure.stderr).contains("E0308"),
        "{}",
        String::from_utf8_lossy(&failure.stderr)
    );
    assert_eq!(fs::read(&source).unwrap(), invalid);
    println!(
        "PRIVATE_RUST_IR_PASS: fixed no_std library compiled to bounded LLVM stdout; real E0308; copied {} runtime bytes; no linker, Cargo, dependency install, native app execution or MCP grant",
        MAX_RUNTIME - remaining
    );
    assert!(!owner.uncertain.get());
    owner.profile.take().unwrap().close().unwrap();
}
