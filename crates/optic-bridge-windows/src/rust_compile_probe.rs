//! Opt-in fixed public Rust compilation proof, never an execution/tool grant.
use crate::{
    AppContainerProfile, AppContainerStdio, CapturedOutputPipe, ContainedAppContainerProcess,
    LimitedJobObject, WindowsFileIdentity, inspect_directory_no_reparse, open_exclusive_read_file,
    open_file_no_reparse, pin_directory_chain, spawn_appcontainer_with_environment_suspended,
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
    selected_files_limited(directory, select, 8)
}

fn selected_files_limited(
    directory: &Path,
    select: impl Fn(&str) -> bool,
    max_files: usize,
) -> io::Result<Vec<PathBuf>> {
    if max_files == 0 || max_files > 64 {
        return Err(io::Error::other("runtime selection count bound"));
    }
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
            if selected.len() == max_files {
                return Err(io::Error::other("too many selected Rust runtime files"));
            }
            selected.push(item.path());
        }
    }
    selected.sort();
    Ok(selected)
}

// Diagnostic metadata only, never a recursive cleanup fallback. Unknown
// compiler products keep the original failure; inspect at most eight names.
fn remove_empty_fixture_directory(path: &Path) {
    if let Err(error) = fs::remove_dir(path) {
        let remaining: Vec<String> = fs::read_dir(path)
            .into_iter()
            .flatten()
            .take(8)
            .map(|entry| match entry {
                Ok(entry) => entry
                    .file_name()
                    .to_string_lossy()
                    .chars()
                    .take(80)
                    .collect(),
                Err(_) => "<entry-unavailable>".to_owned(),
            })
            .collect();
        panic!(
            "fixture cleanup failed: {error}; leaf={:?}; bounded remaining names={remaining:?}",
            path.file_name()
        );
    }
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

fn compile(
    owner: &OwnedProbeProfile,
    root: &Path,
    source: &Path,
    private_output: bool,
) -> io::Result<Compilation> {
    let executable = root.join("bin/rustc.exe");
    let destination = if private_output {
        root.join("output/library.ll").into_os_string()
    } else {
        OsString::from("-")
    };
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
        destination,
        source.into(),
    ];
    run_native(owner, root, &executable, &args)
}

// Fixed test phases only. Each invocation gets a fresh owned one-process Job
// and independent deadline, including the linker and produced native program.
fn run_native(
    owner: &OwnedProbeProfile,
    root: &Path,
    executable: &Path,
    args: &[OsString],
) -> io::Result<Compilation> {
    run_native_with_process_limit(owner, root, executable, args, 1)
}

fn run_native_with_process_limit(
    owner: &OwnedProbeProfile,
    root: &Path,
    executable: &Path,
    args: &[OsString],
    process_limit: u32,
) -> io::Result<Compilation> {
    if !(1..=2).contains(&process_limit) {
        return Err(io::Error::other("fixed proof process bound"));
    }
    let input = File::open("NUL")?;
    let mut stdout = CapturedOutputPipe::new()?;
    let mut stderr = CapturedOutputPipe::new()?;
    let mut environment = vec![
        (
            "SystemRoot".into(),
            std::env::var_os("SystemRoot").ok_or_else(|| io::Error::other("SystemRoot"))?,
        ),
        ("TEMP".into(), root.join("tmp").into_os_string()),
        ("TMP".into(), root.join("tmp").into_os_string()),
        ("LOCALAPPDATA".into(), root.join("local").into_os_string()),
        ("APPDATA".into(), root.join("roaming").into_os_string()),
    ];
    if process_limit == 2 {
        environment.push(("LIB".into(), root.join("std-libs").into_os_string()));
    }
    owner.uncertain.set(true);
    let child = spawn_appcontainer_with_environment_suspended(
        owner.profile.as_ref().unwrap(),
        executable,
        args,
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
        LimitedJobObject::new(process_limit, 1024 * 1024 * 1024, 50)?,
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

// Fixed public test fixture only. This is not an output-root enrollment API.
// The caller must possess this probe's completed compile result. compile returns
// it only after confirming the owned native tree empty, including failure paths.
fn collect_fixed_output(
    owner: &OwnedProbeProfile,
    root: &Path,
    expected_root: WindowsFileIdentity,
    completed: &Compilation,
) -> io::Result<Vec<u8>> {
    if owner.uncertain.get() || completed.exit != 0 {
        return Err(io::Error::other(
            "output requires confirmed successful compilation",
        ));
    }
    let _root_pin = pin_directory_chain(root, None).map_err(io::Error::other)?;
    if inspect_directory_no_reparse(root)
        .map_err(io::Error::other)?
        .identity
        != expected_root
    {
        return Err(io::Error::other("private output root identity changed"));
    }
    let output = pin_directory_chain(root, Some(Path::new("output"))).map_err(io::Error::other)?;
    let expected = output.final_path().join("library.ll");
    // Exclusive opened handle rejects writers and hard-link/reparse aliases.
    let mut file = open_exclusive_read_file(&expected).map_err(io::Error::other)?;
    if !file
        .final_path()
        .to_string_lossy()
        .eq_ignore_ascii_case(&expected.to_string_lossy())
    {
        return Err(io::Error::other("private output path alias"));
    }
    let size = file.file_mut().metadata()?.len();
    if size == 0 || size > MAX_OUTPUT as u64 {
        return Err(io::Error::other("private output size limit"));
    }
    let mut bytes = Vec::new();
    file.file_mut()
        .take(MAX_OUTPUT as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != size {
        return Err(io::Error::other("private output size changed"));
    }
    Ok(bytes)
}

#[test]
#[ignore = "requires an explicitly selected installed direct Rust compiler"]
fn private_rust_compiles_fixed_library_and_reports_real_type_error() {
    run_rust_probe(false, false);
}

#[test]
#[ignore = "requires direct rustc and explicit Windows SDK kernel32 import library"]
fn private_rust_compiles_links_and_executes_fixed_native_program() {
    run_rust_probe(true, false);
}

#[test]
#[ignore = "requires explicit direct Rust, MSVC linker, CRT libraries and SDK"]
fn private_rust_std_compiles_and_executes_fixed_application() {
    run_rust_probe(true, true);
}

fn run_rust_probe(native_program: bool, standard_library: bool) {
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
    let libraries = selected_files_limited(
        &sysroot.join(&lib_relative),
        |name| {
            (standard_library
                || name.starts_with("libcore-")
                || name.starts_with("libcompiler_builtins-"))
                && (name.ends_with(".rlib") || name.ends_with(".rmeta"))
        },
        if standard_library { 64 } else { 8 },
    )
    .unwrap();
    if standard_library {
        assert!(libraries.len() >= 4 && libraries.len() <= 64);
    } else {
        assert_eq!(
            libraries.len(),
            4,
            "one core and compiler_builtins rlib/rmeta pair from selected sysroot"
        );
    }
    for prefix in ["libcore-", "libcompiler_builtins-"] {
        let archive = libraries
            .iter()
            .filter(|file| {
                file.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(prefix)
                    && file.extension().unwrap() == "rlib"
            })
            .collect::<Vec<_>>();
        assert_eq!(archive.len(), 1);
        assert!(
            libraries.contains(&archive[0].with_extension("rmeta")),
            "matching split metadata"
        );
    }
    let plan = crate::EphemeralAppContainerPlan::generate().unwrap();
    let profile_name = plan.name().to_ascii_lowercase();
    let mut owner = OwnedProbeProfile {
        profile: Some(plan.create().unwrap()),
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
    let directories = [
        "bin",
        "lib",
        "lib/rustlib",
        "lib/rustlib/x86_64-pc-windows-msvc",
        "lib/rustlib/x86_64-pc-windows-msvc/lib",
        "tmp",
        "local",
        "roaming",
        "output",
    ];
    for relative in directories {
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
    let result = compile(&owner, &root, &source, false).unwrap();
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
    let failure = compile(&owner, &root, &source, false).unwrap();
    assert_ne!(failure.exit, 0);
    assert!(
        String::from_utf8_lossy(&failure.stderr).contains("E0308"),
        "{}",
        String::from_utf8_lossy(&failure.stderr)
    );
    assert_eq!(fs::read(&source).unwrap(), invalid);
    fs::write(&source, valid).unwrap();
    let restored = compile(&owner, &root, &source, false).unwrap();
    assert_eq!(
        restored.exit,
        0,
        "{}",
        String::from_utf8_lossy(&restored.stderr)
    );
    assert!(String::from_utf8_lossy(&restored.stdout).contains("optic_add"));
    assert_eq!(fs::read(&source).unwrap(), valid);
    // A second real invocation writes one known output in the private profile.
    // No project path is passed as a writable destination.
    let root_identity = inspect_directory_no_reparse(&root).unwrap().identity;
    let on_disk = compile(&owner, &root, &source, true).unwrap();
    assert_eq!(
        on_disk.exit,
        0,
        "{}",
        String::from_utf8_lossy(&on_disk.stderr)
    );
    assert!(on_disk.stdout.is_empty());
    // Collect an actual compiler artifact without requesting write access.
    let artifact = root.join("output/library.ll");
    let original_permissions = fs::metadata(&artifact).unwrap().permissions();
    let mut permissions = original_permissions.clone();
    permissions.set_readonly(true);
    fs::set_permissions(&artifact, permissions.clone()).unwrap();
    let bytes = collect_fixed_output(&owner, &root, root_identity, &on_disk).unwrap();

    fs::set_permissions(&artifact, original_permissions).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("optic_add"));
    assert_eq!(fs::read(&source).unwrap(), valid);
    assert!(collect_fixed_output(&owner, &root, root_identity, &failure).is_err());
    let mut wrong_root = root_identity;
    wrong_root.file_id[0] ^= 1;
    assert!(collect_fixed_output(&owner, &root, wrong_root, &on_disk).is_err());
    // An open writer must prevent collection, even after the compiler exited.
    let writer = OpenOptions::new().write(true).open(&artifact).unwrap();
    assert!(collect_fixed_output(&owner, &root, root_identity, &on_disk).is_err());
    drop(writer);
    let alias = root.join("output/alias.ll");
    fs::hard_link(&artifact, &alias).unwrap();
    assert!(collect_fixed_output(&owner, &root, root_identity, &on_disk).is_err());
    fs::remove_file(&alias).unwrap();
    owner.uncertain.set(true);
    assert!(collect_fixed_output(&owner, &root, root_identity, &on_disk).is_err());
    owner.uncertain.set(false);
    let oversized = OpenOptions::new().write(true).open(&artifact).unwrap();
    oversized.set_len(MAX_OUTPUT as u64 + 1).unwrap();
    drop(oversized);
    assert!(collect_fixed_output(&owner, &root, root_identity, &on_disk).is_err());
    let empty = OpenOptions::new().write(true).open(&artifact).unwrap();
    empty.set_len(0).unwrap();
    drop(empty);
    assert!(collect_fixed_output(&owner, &root, root_identity, &on_disk).is_err());
    fs::remove_file(&artifact).unwrap();
    assert!(collect_fixed_output(&owner, &root, root_identity, &on_disk).is_err());
    // Reparse output cannot redirect collection to the fixed source fixture.
    std::os::windows::fs::symlink_file(&source, &artifact).unwrap();
    assert!(collect_fixed_output(&owner, &root, root_identity, &on_disk).is_err());
    fs::remove_file(&artifact).unwrap();
    println!(
        "PRIVATE_RUST_FILE_OUTPUT_PASS: actual private LLVM file collected only after owned tree exit; oversized, writer, hardlink, reparse, missing and uncertain cases refused; no disk quota or generic output authority claimed"
    );
    if native_program {
        // Observed Microsoft linker temp virtualization beneath the explicitly
        // supplied private LOCALAPPDATA. Create only this fresh profile's exact
        // known chain, never a directory beneath the operator's real profile.
        let package = PathBuf::from("local/Packages").join(&profile_name);
        let private_temp_directories = [
            PathBuf::from("local/Packages"),
            package.clone(),
            package.join("AC"),
            package.join("AC/Temp"),
        ];
        for relative in &private_temp_directories {
            fs::create_dir(root.join(relative)).unwrap();
        }
        let sdk = PathBuf::from(
            std::env::var_os("OPTIC_PRIVATE_KERNEL32_LIB")
                .expect("explicit fixed Windows SDK kernel32.lib"),
        );
        assert!(sdk.is_absolute());
        assert_eq!(
            sdk.file_name()
                .unwrap()
                .to_string_lossy()
                .to_ascii_lowercase(),
            "kernel32.lib"
        );
        let linker_source = PathBuf::from(
            std::env::var_os("OPTIC_PRIVATE_LINKER").expect("explicit native linker selection"),
        );
        assert!(linker_source.is_absolute());
        let flavor = linker_source
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_ascii_lowercase();
        assert!(matches!(flavor.as_str(), "link.exe" | "rust-lld.exe"));
        let linker_root = root.join("bin/native-linker");
        fs::create_dir(&linker_root).unwrap();
        let linker = linker_root.join(&flavor);
        let linker_dlls = if flavor == "link.exe" {
            selected_files(linker_source.parent().unwrap(), |name| {
                matches!(
                    name.to_ascii_lowercase().as_str(),
                    "vcruntime140.dll"
                        | "vcruntime140_1.dll"
                        | "msvcp140.dll"
                        | "msvcp140_atomic_wait.dll"
                        | "tbbmalloc.dll"
                        | "msobj140.dll"
                        | "mspdbcore.dll"
                        | "mspdb140.dll"
                )
            })
            .unwrap()
        } else {
            Vec::new()
        };
        for dll in &linker_dlls {
            copy_exact(
                dll,
                &linker_root.join(dll.file_name().unwrap()),
                &mut remaining,
            )
            .unwrap();
        }
        let import = root.join("output/kernel32.lib");
        copy_exact(&linker_source, &linker, &mut remaining).unwrap();
        copy_exact(&sdk, &import, &mut remaining).unwrap();
        let native_source = b"#![no_std]\n#[link(name=\"kernel32\")] unsafe extern \"system\" { fn ExitProcess(code:u32)->!; }\n#[unsafe(no_mangle)] pub extern \"system\" fn optic_entry()->! { unsafe { ExitProcess(42) } }\n#[panic_handler] fn panic(_: &core::panic::PanicInfo)->! { loop {} }\n";
        fs::write(&source, native_source).unwrap();
        let object = root.join("output/native.obj");
        let executable = root.join("output/native.exe");
        assert!(!object.exists() && !executable.exists());
        let object_args: Vec<OsString> = vec![
            "--crate-name".into(),
            "optic_native_probe".into(),
            "--crate-type=lib".into(),
            "--edition=2024".into(),
            "--target".into(),
            TARGET.into(),
            "--sysroot".into(),
            root.clone().into_os_string(),
            "--emit=obj".into(),
            "-Cdebuginfo=0".into(),
            "-Cpanic=abort".into(),
            "-o".into(),
            object.clone().into_os_string(),
            source.clone().into_os_string(),
        ];
        let compiled =
            run_native(&owner, &root, &root.join("bin/rustc.exe"), &object_args).unwrap();
        assert_eq!(
            compiled.exit,
            0,
            "compiler stdout: {}\ncompiler stderr: {}",
            String::from_utf8_lossy(&compiled.stdout),
            String::from_utf8_lossy(&compiled.stderr)
        );
        assert!(compiled.stdout.is_empty());
        assert!(fs::metadata(&object).unwrap().len() <= MAX_OUTPUT as u64);
        let mut link_args: Vec<OsString> = vec![
            "/entry:optic_entry".into(),
            "/subsystem:console".into(),
            "/nodefaultlib".into(),
            "/manifest:no".into(),
            "/incremental:no".into(),
            "/machine:x64".into(),
            format!("/out:{}", executable.display()).into(),
            object.clone().into_os_string(),
            import.clone().into_os_string(),
        ];
        if flavor == "rust-lld.exe" {
            link_args.splice(0..0, [OsString::from("-flavor"), OsString::from("link")]);
        } else {
            link_args.insert(0, "/nologo".into());
        }
        let linked = run_native(&owner, &root, &linker, &link_args).unwrap();
        assert_eq!(
            linked.exit,
            0,
            "linker stdout: {}\nlinker stderr: {}",
            String::from_utf8_lossy(&linked.stdout),
            String::from_utf8_lossy(&linked.stderr)
        );
        assert!(fs::metadata(&executable).unwrap().len() <= MAX_OUTPUT as u64);
        let executed = run_native(&owner, &root, &executable, &[]).unwrap();
        assert_eq!(
            executed.exit,
            42,
            "{}",
            String::from_utf8_lossy(&executed.stderr)
        );
        assert!(executed.stdout.is_empty() && executed.stderr.is_empty());
        assert_eq!(fs::read(&source).unwrap(), native_source);
        if standard_library {
            assert_eq!(
                flavor, "link.exe",
                "explicit Microsoft standard-library proof profile"
            );
            let crt = PathBuf::from(
                std::env::var_os("OPTIC_PRIVATE_CRT_LIB_DIR")
                    .expect("explicit CRT library directory"),
            );
            assert!(crt.is_absolute());
            let native_libs = root.join("std-libs");
            fs::create_dir(&native_libs).unwrap();
            let mut copied_libs = Vec::new();
            for name in [
                "msvcrt.lib",
                "vcruntime.lib",
                "legacy_stdio_definitions.lib",
            ] {
                let destination = native_libs.join(name);
                copy_exact(&crt.join(name), &destination, &mut remaining).unwrap();
                copied_libs.push(destination);
            }
            for name in [
                "kernel32.lib",
                "advapi32.lib",
                "bcrypt.lib",
                "ntdll.lib",
                "userenv.lib",
                "ws2_32.lib",
                "dbghelp.lib",
                "synchronization.lib",
            ] {
                let destination = native_libs.join(name);
                copy_exact(
                    &sdk.parent().unwrap().join(name),
                    &destination,
                    &mut remaining,
                )
                .unwrap();
                copied_libs.push(destination);
            }
            let ucrt = PathBuf::from(
                std::env::var_os("OPTIC_PRIVATE_UCRT_LIB").expect("explicit UCRT import library"),
            );
            assert!(ucrt.is_absolute());
            let destination = native_libs.join("ucrt.lib");
            copy_exact(&ucrt, &destination, &mut remaining).unwrap();
            copied_libs.push(destination);
            let mut app_dlls = Vec::new();
            for dll in &linker_dlls {
                let name = dll
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_ascii_lowercase();
                if matches!(name.as_str(), "vcruntime140.dll" | "vcruntime140_1.dll") {
                    let destination = root.join("output").join(dll.file_name().unwrap());
                    copy_exact(dll, &destination, &mut remaining).unwrap();
                    app_dlls.push(destination);
                }
            }
            let standard_source = b"fn main() { let values: Vec<i32> = [\"2\", \"3\"].iter().map(|s| s.parse().unwrap()).collect(); assert_eq!(values.iter().sum::<i32>(), 5); println!(\"PRIVATE_RUST_STD_APP_PASS\"); }\n";
            fs::write(&source, standard_source).unwrap();
            let standard_exe = root.join("output/std-application.exe");
            let args: Vec<OsString> = vec![
                "--crate-name=optic_std_probe".into(),
                "--crate-type=bin".into(),
                "--edition=2024".into(),
                "--target".into(),
                TARGET.into(),
                "--sysroot".into(),
                root.clone().into_os_string(),
                "-Cdebuginfo=0".into(),
                "-Cpanic=abort".into(),
                format!("-Clinker={}", linker.display()).into(),
                "-Clink-arg=/manifest:no".into(),
                "-Clink-arg=/incremental:no".into(),
                "-L".into(),
                format!("native={}", native_libs.display()).into(),
                "-o".into(),
                standard_exe.clone().into_os_string(),
                source.clone().into_os_string(),
            ];
            // The driver's linker is an actual descendant in the same owned Job.
            let built =
                run_native_with_process_limit(&owner, &root, &root.join("bin/rustc.exe"), &args, 2)
                    .unwrap();
            assert_eq!(
                built.exit,
                0,
                "stdout: {}\nstderr: {}",
                String::from_utf8_lossy(&built.stdout),
                String::from_utf8_lossy(&built.stderr)
            );
            assert!(fs::metadata(&standard_exe).unwrap().len() <= 8 * 1024 * 1024);
            let app = run_native(&owner, &root, &standard_exe, &[]).unwrap();
            assert_eq!(app.exit, 0, "{}", String::from_utf8_lossy(&app.stderr));
            assert_eq!(app.stdout, b"PRIVATE_RUST_STD_APP_PASS\n");
            assert!(app.stderr.is_empty());
            assert_eq!(fs::read(&source).unwrap(), standard_source);
            fs::remove_file(&standard_exe).unwrap();
            for recorded in copied_libs.into_iter().chain(app_dlls) {
                fs::remove_file(recorded).unwrap();
            }
            fs::remove_dir(&native_libs).unwrap();
            println!(
                "PRIVATE_RUST_STD_PASS: actual std binary compiled with owned linker descendant, executed Vec/parse/assert/println successfully, private copies only; no Cargo/dependency install/generic build or disk quota claimed"
            );
        }
        for recorded in [&executable, &object, &import, &linker] {
            fs::remove_file(recorded).unwrap();
        }
        for dll in &linker_dlls {
            fs::remove_file(linker_root.join(dll.file_name().unwrap())).unwrap();
        }
        fs::remove_dir(&linker_root).unwrap();
        for relative in private_temp_directories.iter().rev() {
            remove_empty_fixture_directory(&root.join(relative));
        }
        println!(
            "PRIVATE_RUST_NATIVE_PASS: actual no_std COFF object compiled, explicitly selected {flavor} linked against copied kernel32 import, generated native app exited42 under owned AppContainer Jobs; no Cargo, CRT, generic build or disk quota claimed"
        );
    }
    // Delete recorded fixed fixture files only, after every native tree was
    // confirmed empty. Unknown compiler files make empty-directory removal fail.
    fs::remove_file(&source).unwrap();
    fs::remove_file(root.join("bin/rustc.exe")).unwrap();
    for dll in &dlls {
        fs::remove_file(root.join("bin").join(dll.file_name().unwrap())).unwrap();
    }
    for library in &libraries {
        fs::remove_file(root.join(&lib_relative).join(library.file_name().unwrap())).unwrap();
    }
    for relative in directories.into_iter().rev() {
        remove_empty_fixture_directory(&root.join(relative));
    }
    fs::remove_dir(&root).unwrap();
    println!(
        "PRIVATE_RUST_IR_PASS: fixed no_std library compiled to bounded LLVM stdout; real E0308 then corrected recompile; copied {} runtime bytes; recorded files cleaned; native variant={native_program}; no Cargo, dependency install or MCP grant",
        MAX_RUNTIME - remaining
    );
    assert!(!owner.uncertain.get());
    owner.profile.take().unwrap().close().unwrap();
}
