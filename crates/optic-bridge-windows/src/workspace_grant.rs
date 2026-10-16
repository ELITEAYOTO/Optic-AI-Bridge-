use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    io::{Error, Result},
    marker::PhantomData,
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::Path,
    ptr,
};

use windows::Win32::{
    Foundation::{HANDLE, HLOCAL, LocalFree},
    Security::{
        ACL,
        Authorization::{
            BuildTrusteeWithSidW, EXPLICIT_ACCESS_W, GRANT_ACCESS, GetSecurityInfo, REVOKE_ACCESS,
            SE_FILE_OBJECT, SetEntriesInAclW, SetSecurityInfo, TRUSTEE_W,
        },
        DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
    },
    Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE, FileAttributeTagInfo,
        GetFileInformationByHandleEx,
    },
};

use crate::{
    appcontainer::AppContainerProfile,
    file::{inspect_directory_no_reparse, query_final_path},
};

const WRITE_DAC_ACCESS: u32 = 0x0004_0000;
const READ_CONTROL_ACCESS: u32 = 0x0002_0000;
const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;

impl AppContainerProfile {
    /// Permit metadata/traversal of one pinned directory inside a pinned root.
    /// No listing, file-content, write or inheritable permission is granted.
    pub fn grant_directory_metadata_within<'a>(
        &'a self,
        path: &Path,
        workspace_root: &Path,
    ) -> Result<AppContainerDirectoryMetadataGrant<'a>> {
        if !path.is_absolute() || !workspace_root.is_absolute() {
            return Err(Error::other("metadata grant paths must be absolute"));
        }
        if !self.external_acl_grants_allowed() {
            return Err(Error::other(
                "workspace grants require an ephemeral profile",
            ));
        }
        let root = open_metadata_directory(workspace_root)?;
        let root_path = query_final_path(&root).map_err(Error::other)?;
        let directory = open_metadata_directory(path)?;
        let final_path = query_final_path(&directory).map_err(Error::other)?;
        if !final_path.starts_with(&root_path) {
            return Err(Error::other(
                "metadata directory resolves outside workspace",
            ));
        }
        let sid = self.package_sid();
        modify_file_dacl(
            &directory,
            sid,
            true,
            FILE_READ_ATTRIBUTES.0 | FILE_TRAVERSE.0 | SYNCHRONIZE_ACCESS,
        )?;
        Ok(AppContainerDirectoryMetadataGrant {
            directory,
            _root: root,
            sid,
            active: true,
            _profile: PhantomData,
        })
    }

    /// Grant this fresh AppContainer profile read-only access to one exact file.
    ///
    /// The profile Package SID is already part of every process token created for
    /// this AppContainer, so no additional capability SID or network authority is
    /// introduced. The target is opened handle-first with final reparse points
    /// denied. The ACE is non-inheritable and is revoked on explicit `revoke` or
    /// best-effort Drop.
    pub fn grant_file_read<'a>(&'a self, path: &Path) -> Result<AppContainerReadFileGrant<'a>> {
        if !self.external_acl_grants_allowed() {
            return Err(Error::other(
                "workspace grants require a cryptographically ephemeral AppContainer profile",
            ));
        }
        AppContainerReadFileGrant::new(path, self, None)
    }

    /// Grant read-only access only when the exact opened target resolves under
    /// the exact opened workspace root. Both decisions are handle-bound.
    pub fn grant_file_read_within<'a>(
        &'a self,
        path: &Path,
        workspace_root: &Path,
    ) -> Result<AppContainerReadFileGrant<'a>> {
        if !self.external_acl_grants_allowed() {
            return Err(Error::other(
                "workspace grants require a cryptographically ephemeral AppContainer profile",
            ));
        }
        let root = inspect_directory_no_reparse(workspace_root).map_err(Error::other)?;
        AppContainerReadFileGrant::new(path, self, Some(&root.final_path))
    }
}

/// Handle-bound read-only grant for one exact file object.
#[derive(Debug)]
pub struct AppContainerReadFileGrant<'a> {
    file: File,
    sid: PSID,
    active: bool,
    _profile: PhantomData<&'a AppContainerProfile>,
}

impl<'a> AppContainerReadFileGrant<'a> {
    fn new(
        path: &Path,
        profile: &'a AppContainerProfile,
        workspace_root: Option<&Path>,
    ) -> Result<Self> {
        let mut options = OpenOptions::new();
        options
            .access_mode(FILE_GENERIC_READ.0 | WRITE_DAC_ACCESS)
            .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0);
        let file = options.open(path)?;
        reject_reparse(&file)?;
        if let Some(workspace_root) = workspace_root {
            let final_path = query_final_path(&file).map_err(Error::other)?;
            if !final_path.starts_with(workspace_root) {
                return Err(Error::other(
                    "workspace grant target resolves outside the configured workspace",
                ));
            }
        }
        let sid = profile.package_sid();
        modify_file_dacl(&file, sid, true, FILE_GENERIC_READ.0)?;
        Ok(Self {
            file,
            sid,
            active: true,
            _profile: PhantomData,
        })
    }

    /// Revoke the exact Package SID ACE. Consuming self prevents accidental reuse.
    pub fn revoke(mut self) -> Result<()> {
        self.revoke_inner()
    }

    fn revoke_inner(&mut self) -> Result<()> {
        if self.active {
            modify_file_dacl(&self.file, self.sid, false, FILE_GENERIC_READ.0)?;
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for AppContainerReadFileGrant<'_> {
    fn drop(&mut self) {
        let _ = self.revoke_inner();
    }
}

/// Pins both directory objects without delete sharing for the grant lifetime.
#[derive(Debug)]
pub struct AppContainerDirectoryMetadataGrant<'a> {
    directory: File,
    _root: File,
    sid: PSID,
    active: bool,
    _profile: PhantomData<&'a AppContainerProfile>,
}

impl AppContainerDirectoryMetadataGrant<'_> {
    /// Revoke the exact SID entry and release both namespace pins.
    pub fn revoke(mut self) -> Result<()> {
        self.revoke_inner()
    }

    fn revoke_inner(&mut self) -> Result<()> {
        if self.active {
            modify_file_dacl(&self.directory, self.sid, false, 0)?;
            self.active = false;
        }
        Ok(())
    }
}

impl Drop for AppContainerDirectoryMetadataGrant<'_> {
    fn drop(&mut self) {
        let _ = self.revoke_inner();
    }
}

fn open_metadata_directory(path: &Path) -> Result<File> {
    let directory = OpenOptions::new()
        .access_mode(
            READ_CONTROL_ACCESS | WRITE_DAC_ACCESS | FILE_READ_ATTRIBUTES.0 | FILE_TRAVERSE.0,
        )
        .share_mode(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)?;
    reject_reparse(&directory)?;
    if !directory.metadata()?.is_dir() {
        return Err(Error::other("metadata grant requires a directory"));
    }
    Ok(directory)
}

fn modify_file_dacl(file: &File, sid: PSID, grant: bool, permissions: u32) -> Result<()> {
    let handle = HANDLE(file.as_raw_handle());
    let mut old_acl = ptr::null_mut::<ACL>();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // SAFETY: handle is a live exact-object handle opened with READ_CONTROL.
    // Returned DACL aliases the returned LocalAlloc descriptor.
    let status = unsafe {
        GetSecurityInfo(
            handle,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut old_acl),
            None,
            Some(&mut descriptor),
        )
    };
    win32_status(status.0, "GetSecurityInfo")?;
    let _descriptor = LocalAllocation(descriptor.0);
    if old_acl.is_null() {
        return Err(Error::other(
            "workspace grant refuses a null or absent DACL",
        ));
    }

    let mut trustee = TRUSTEE_W::default();
    // SAFETY: SID is owned by the live AppContainer profile and remains valid throughout.
    unsafe { BuildTrusteeWithSidW(&mut trustee, Some(sid)) };
    let entry = EXPLICIT_ACCESS_W {
        grfAccessPermissions: if grant { permissions } else { 0 },
        grfAccessMode: if grant { GRANT_ACCESS } else { REVOKE_ACCESS },
        grfInheritance: Default::default(),
        Trustee: trustee,
    };

    let mut new_acl = ptr::null_mut::<ACL>();
    // SAFETY: old ACL remains live through `_descriptor`; entry and output
    // storage are valid for the synchronous merge call.
    let status = unsafe {
        SetEntriesInAclW(
            Some(std::slice::from_ref(&entry)),
            (!old_acl.is_null()).then_some(old_acl.cast_const()),
            &mut new_acl,
        )
    };
    win32_status(status.0, "SetEntriesInAclW")?;
    let _new_acl = LocalAllocation(new_acl.cast::<c_void>());

    // SAFETY: the exact file handle is live and was opened with WRITE_DAC;
    // `new_acl` stays allocated through the synchronous update.
    let status = unsafe {
        SetSecurityInfo(
            handle,
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(new_acl.cast_const()),
            None,
        )
    };
    win32_status(status.0, "SetSecurityInfo")
}

fn reject_reparse(file: &File) -> Result<()> {
    let handle = HANDLE(file.as_raw_handle());
    let mut info = FILE_ATTRIBUTE_TAG_INFO::default();
    // SAFETY: handle is live and output storage has the exact Windows type/size.
    unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            (&mut info as *mut FILE_ATTRIBUTE_TAG_INFO).cast::<c_void>(),
            u32::try_from(std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>())
                .expect("FILE_ATTRIBUTE_TAG_INFO size must fit u32"),
        )
    }
    .map_err(Error::other)?;
    if info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(Error::other(
            "workspace grant target cannot be a reparse point",
        ));
    }
    Ok(())
}

fn win32_status(code: u32, operation: &str) -> Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(Error::other(format!(
            "{operation} failed with Win32 error {code}"
        )))
    }
}

#[derive(Debug)]
struct LocalAllocation(*mut c_void);

impl Drop for LocalAllocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: pointer came from a Win32 LocalAlloc-returning API and is freed once.
            unsafe {
                let _ = LocalFree(Some(HLOCAL(self.0)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        fs::{self, OpenOptions},
        io::Write as _,
        os::windows::{fs::OpenOptionsExt, io::AsHandle},
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    use crate::{
        AppContainerProfile, AppContainerStdio, spawn_appcontainer_with_environment_suspended,
    };
    use windows::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
    };

    static SEQUENCE: AtomicU64 = AtomicU64::new(1);
    const WAIT_MS: u32 = 10_000;
    const PROBE_CONFIG: &str = "optic-workspace-grant-probe.txt";
    const PROBE_SENTINEL: &str = "optic read grant sentinel\n";

    fn unique_profile_name() -> String {
        format!(
            "Optic.Grant.Test.{}.{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        )
    }

    fn unique_temp_file() -> PathBuf {
        std::env::temp_dir().join(format!(
            "optic-appcontainer-read-grant-{}-{}.txt",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[derive(Debug)]
    struct IsolatedOutput {
        exit_code: u32,
        stdout: String,
        stderr: String,
    }

    impl IsolatedOutput {
        fn diagnostic(&self) -> String {
            format!(
                "exit={} stdout={:?} stderr={:?}",
                self.exit_code, self.stdout, self.stderr
            )
        }
    }

    fn run_isolated_capture(
        profile: &AppContainerProfile,
        executable: &Path,
        args: Vec<OsString>,
    ) -> IsolatedOutput {
        let cwd = executable.parent().expect("isolated executable parent");
        run_isolated_capture_at(profile, executable, args, cwd)
    }

    fn run_isolated_capture_at(
        profile: &AppContainerProfile,
        executable: &Path,
        args: Vec<OsString>,
        cwd: &Path,
    ) -> IsolatedOutput {
        run_isolated_capture_with_budget(profile, executable, args, cwd, WAIT_MS)
    }

    fn run_isolated_capture_with_budget(
        profile: &AppContainerProfile,
        executable: &Path,
        args: Vec<OsString>,
        cwd: &Path,
        timeout_ms: u32,
    ) -> IsolatedOutput {
        let stdin = OpenOptions::new()
            .read(true)
            .open("NUL")
            .expect("NUL stdin");
        let stdout_path = unique_temp_file();
        let stderr_path = unique_temp_file();
        let _stdout_cleanup = Cleanup(stdout_path.clone());
        let _stderr_cleanup = Cleanup(stderr_path.clone());
        let stdout = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&stdout_path)
            .expect("capture stdout");
        let stderr = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&stderr_path)
            .expect("capture stderr");
        let private = profile
            .private_storage_root()
            .expect("native private environment root");
        for name in ["tmp", "local", "roaming"] {
            let folder = private.join(name);
            if !folder.exists() {
                fs::create_dir(&folder).expect("private environment directory");
            }
            crate::inspect_directory_no_reparse(&folder)
                .expect("ordinary private environment directory");
        }
        let environment = vec![
            (
                OsString::from("SystemRoot"),
                std::env::var_os("SystemRoot").expect("SystemRoot"),
            ),
            (
                OsString::from("LOCALAPPDATA"),
                private.join("local").into_os_string(),
            ),
            (
                OsString::from("APPDATA"),
                private.join("roaming").into_os_string(),
            ),
            (OsString::from("TEMP"), private.join("tmp").into_os_string()),
            (OsString::from("TMP"), private.join("tmp").into_os_string()),
            (OsString::from("OPTIC_PRIVATE_ENV"), OsString::from("1")),
        ];
        let mut child = spawn_appcontainer_with_environment_suspended(
            profile,
            executable,
            &args,
            cwd,
            AppContainerStdio {
                stdin: stdin.as_handle(),
                stdout: stdout.as_handle(),
                stderr: stderr.as_handle(),
            },
            &environment,
        )
        .expect("spawn AppContainer");
        assert!(child.is_appcontainer().expect("query AppContainer token"));
        let job = crate::job::AppContainerProbeJob::assign(child.borrowed_process_for_tests())
            .expect("contain owned suspended probe and descendants");
        child.resume().expect("resume AppContainer");
        let exit_code = child.wait_exit(timeout_ms);
        // Confirm the entire tree is gone before reading outputs or deleting
        // private stage files; a completed root alone is insufficient.
        job.terminate_and_confirm_empty()
            .expect("empty owned probe tree");
        let exit_code = exit_code.unwrap_or(125);
        drop(stdout);
        drop(stderr);
        IsolatedOutput {
            exit_code,
            stdout: read_probe_output(&stdout_path),
            stderr: read_probe_output(&stderr_path),
        }
    }

    fn read_probe_output(path: &Path) -> String {
        use std::io::Read;
        let mut bytes = Vec::new();
        fs::File::open(path)
            .expect("probe output")
            .take(64 * 1024 + 1)
            .read_to_end(&mut bytes)
            .expect("bounded probe output read");
        assert!(bytes.len() <= 64 * 1024, "probe output limit");
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn write_probe_config(profile_dir: &Path, mode: &str, target: &Path) {
        let target = target.to_string_lossy();
        assert!(
            !target.contains(['\r', '\n']),
            "probe target must fit one config line"
        );
        fs::write(
            profile_dir.join(PROBE_CONFIG),
            format!("{mode}\n{target}\n"),
        )
        .expect("write probe config");
    }

    fn run_workspace_probe(
        profile: &AppContainerProfile,
        probe_exe: &Path,
        target: &Path,
        mode: &str,
    ) -> IsolatedOutput {
        let profile_dir = probe_exe.parent().expect("probe profile directory");
        write_probe_config(profile_dir, mode, target);
        run_isolated_capture(
            profile,
            probe_exe,
            vec![
                OsString::from("--exact"),
                OsString::from("workspace_grant::tests::appcontainer_workspace_probe_child"),
                OsString::from("--nocapture"),
                OsString::from("--test-threads=1"),
            ],
        )
    }

    #[test]
    fn appcontainer_workspace_probe_child() {
        let exact_child = std::env::args().any(|arg| arg == "--exact");
        let config_path = Path::new(PROBE_CONFIG);
        if !exact_child || !config_path.is_file() {
            return;
        }

        let config = fs::read_to_string(config_path).expect("read workspace probe config");
        let mut lines = config.lines();
        let mode = lines.next().expect("probe mode");
        let target = PathBuf::from(lines.next().expect("probe target"));
        assert!(
            lines.next().is_none(),
            "probe config must contain two lines"
        );

        match mode {
            "metadata" => {
                // Request only the declared attribute rights. std::fs::metadata
                // may use a broader directory-open fallback under AppContainer.
                let directory = OpenOptions::new()
                    .access_mode(FILE_READ_ATTRIBUTES.0)
                    .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0 | FILE_FLAG_OPEN_REPARSE_POINT.0)
                    .open(&target)
                    .expect("open directory for attributes only");
                assert!(directory.metadata().expect("handle metadata").is_dir());
            }
            "list" => {
                let _ = fs::read_dir(&target).expect("directory listing");
            }
            "create" => fs::write(target.join("forbidden-create.txt"), b"forbidden")
                .expect("directory create"),
            "read" => {
                let content = fs::read_to_string(&target).expect("probe read exact file");
                assert_eq!(content, PROBE_SENTINEL);
            }
            "append" => {
                let mut file = OpenOptions::new()
                    .append(true)
                    .open(&target)
                    .expect("probe open exact file for append");
                file.write_all(b"forbidden-write\n")
                    .expect("probe append exact file");
            }
            other => panic!("unsupported workspace probe mode: {other}"),
        }
    }

    #[test]
    fn named_profile_cannot_mint_workspace_grant() {
        let sentinel = unique_temp_file();
        fs::write(&sentinel, b"optic read grant sentinel\n").expect("write sentinel");
        let _cleanup = Cleanup(sentinel.clone());
        let profile = AppContainerProfile::create(&unique_profile_name()).expect("create profile");
        assert!(profile.private_storage_root().is_err());

        assert!(
            profile.grant_file_read(&sentinel).is_err(),
            "predictably named profiles must not mint persistent filesystem grants"
        );
        assert!(
            profile
                .grant_directory_metadata_within(
                    sentinel.parent().unwrap(),
                    sentinel.parent().unwrap()
                )
                .is_err()
        );
    }

    #[test]
    #[ignore = "requires an explicitly selected installed Node; run the native development-stage gate"]
    fn private_stage_node_modules_can_write_artifacts_but_not_user_sources() {
        let selected = std::env::var_os("OPTIC_STAGE_NODE").expect("select Node explicitly");
        let node = fs::canonicalize(selected).expect("installed Node");
        assert!(node.is_file());
        assert!(
            node.file_name()
                .unwrap()
                .to_string_lossy()
                .eq_ignore_ascii_case("node.exe")
        );
        let sentinel = unique_temp_file();
        fs::write(&sentinel, PROBE_SENTINEL).expect("ungranted source sentinel");
        let _sentinel_cleanup = Cleanup(sentinel.clone());
        let profile = AppContainerProfile::create_ephemeral().expect("private staging profile");
        let profile_dir = profile.profile_storage_path_for_tests();
        let stage = profile_dir.join("bounded-build-stage");
        assert!(!stage.exists());
        fs::create_dir(&stage).expect("stage root");
        fs::create_dir(stage.join("lib")).expect("staged module directory");
        fs::write(stage.join("lib/math.cjs"), b"exports.add=(a,b)=>a+b;\n").expect("staged module");
        fs::write(stage.join("package.json"), b"{\"type\":\"commonjs\"}").expect("staged package");
        fs::write(
            stage.join("main.cjs"),
            br#"
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {add}=require('./lib/math.cjs');
assert.equal(process.env.OPTIC_PRIVATE_ENV,'1');
assert.equal(process.env.USERPROFILE,undefined);assert.equal(process.env.HOME,undefined);
assert.equal(add(2,3),5);
assert.equal(process.cwd(),__dirname);
fs.mkdirSync(path.join(__dirname,'dist'));
fs.writeFileSync(path.join(__dirname,'dist/result.json'),JSON.stringify({sum:add(2,3)}));
let denied=false;
try{fs.writeFileSync(process.argv[2],'MUST_NOT_REPLACE_USER_SOURCE');}
catch(error){denied=error.code==='EACCES'||error.code==='EPERM';}
assert.equal(denied,true);
console.log('PRIVATE_STAGE_NODE_PASS');
"#,
        )
        .expect("bounded build program");
        let output = run_isolated_capture_at(
            &profile,
            &node,
            vec![
                OsString::from("--preserve-symlinks-main"),
                OsString::from("--preserve-symlinks"),
                stage.join("main.cjs").into_os_string(),
                sentinel.clone().into_os_string(),
            ],
            &stage,
        );
        assert_eq!(output.exit_code, 0, "{}", output.diagnostic());
        assert!(output.stdout.contains("PRIVATE_STAGE_NODE_PASS"));
        assert_eq!(
            fs::read(stage.join("dist/result.json")).expect("artifact"),
            b"{\"sum\":5}"
        );
        assert_eq!(
            fs::read_to_string(&sentinel).expect("unchanged user source"),
            PROBE_SENTINEL
        );
        assert!(
            fs::canonicalize(&stage)
                .expect("stage identity")
                .starts_with(fs::canonicalize(&profile_dir).expect("profile root"))
        );
        fs::remove_dir_all(&stage).expect("remove test-owned staged inputs/artifact");
        println!(
            "PRIVATE_STAGE_NODE_PASS: staged relative modules, private build output, ungranted source writes denied; no new runtime grants"
        );
    }

    /// This is an explicit compatibility gate, never enabled by core startup.
    #[test]
    #[ignore = "requires an explicitly selected installed Electron runtime and a Windows desktop"]
    fn private_stage_electron_window_preserves_outer_appcontainer_boundary() {
        let selected =
            std::env::var_os("OPTIC_STAGE_ELECTRON").expect("select installed Electron explicitly");
        let electron = fs::canonicalize(selected).expect("installed Electron");
        assert!(
            electron
                .file_name()
                .unwrap()
                .to_string_lossy()
                .eq_ignore_ascii_case("electron.exe")
        );
        let sentinel = unique_temp_file();
        fs::write(&sentinel, PROBE_SENTINEL).expect("external source sentinel");
        let _sentinel_cleanup = Cleanup(sentinel.clone());
        let profile = AppContainerProfile::create_ephemeral().expect("private Electron profile");
        let profile_dir = profile.profile_storage_path_for_tests();
        let stage = profile_dir.join("electron-compatibility-stage");
        fs::create_dir(&stage).expect("fresh private stage");
        let runtime = stage.join("runtime");
        fs::create_dir(&runtime).expect("private runtime root");
        copy_electron_runtime_for_probe(electron.parent().unwrap(), &runtime);
        let application = stage.join("application");
        fs::create_dir(&application).expect("application root");
        fs::write(
            application.join("package.json"),
            b"{\"name\":\"optic-private-probe\",\"main\":\"main.cjs\"}",
        )
        .expect("package");
        fs::write(application.join("main.cjs"), br#"
'use strict';
const {app,BrowserWindow}=require('electron');
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
assert.equal(process.env.OPTIC_PRIVATE_ENV,'1');
assert.equal(process.env.USERPROFILE,undefined);assert.equal(process.env.HOME,undefined);
app.disableHardwareAcceleration();
app.setPath('userData',path.join(__dirname,'state'));
app.setPath('logs',path.join(__dirname,'logs'));
const timer=setTimeout(()=>app.exit(3),20000);
app.whenReady().then(async()=>{
  const window=new BrowserWindow({show:false,webPreferences:{sandbox:true,nodeIntegration:false,contextIsolation:true}});
  await window.loadURL('data:text/html,<html><body><h1>Private Electron probe</h1></body></html>');
  assert.equal(await window.webContents.executeJavaScript('6*7'),42);
  assert.equal(await window.webContents.executeJavaScript('typeof require'),'undefined');
  assert.equal(await window.webContents.executeJavaScript('typeof process'),'undefined');
  let readDenied=false,writeDenied=false;
  try{fs.readFileSync(process.argv.at(-1));}catch(e){readDenied=e.code==='EACCES'||e.code==='EPERM';}
  try{fs.writeFileSync(process.argv.at(-1),'MUST_NOT_REPLACE_USER_SOURCE');}catch(e){writeDenied=e.code==='EACCES'||e.code==='EPERM';}
  assert.equal(readDenied,true);assert.equal(writeDenied,true);
  fs.writeFileSync(path.join(__dirname,'result.json'),JSON.stringify({renderer:42,sourceReadDenied:readDenied,sourceWriteDenied:writeDenied}));
  console.log('PRIVATE_STAGE_ELECTRON_PASS');
  clearTimeout(timer);window.destroy();app.quit();
}).catch(error=>{console.error(String(error));app.exit(2);});
"#).expect("fixed compatibility application");
        let mut arguments = vec![
            OsString::from("--enable-logging=file"),
            OsString::from(format!(
                "--log-file={}",
                application.join("startup.log").display()
            )),
            OsString::from(format!(
                "--user-data-dir={}",
                application.join("state").display()
            )),
            OsString::from("--v=1"),
        ];
        match std::env::var("OPTIC_PRIVATE_ELECTRON_VARIANT").as_deref() {
            Ok("no-stdio-init") => arguments.push(OsString::from("--no-stdio-init")),
            Ok("default") | Err(_) => {}
            _ => panic!("unsupported fixed compatibility variant"),
        }
        arguments.push(application.clone().into_os_string());
        arguments.push(sentinel.clone().into_os_string());
        let output = run_isolated_capture_with_budget(
            &profile,
            &runtime.join("electron.exe"),
            arguments,
            &application,
            30_000,
        );
        let startup_log = application.join("startup.log");
        if startup_log.is_file() {
            println!(
                "PRIVATE_ELECTRON_STARTUP_LOG: {}",
                read_probe_output(&startup_log)
            );
        }
        assert_eq!(output.exit_code, 0, "{}", output.diagnostic());
        assert!(
            output.stdout.contains("PRIVATE_STAGE_ELECTRON_PASS"),
            "{}",
            output.diagnostic()
        );
        assert_eq!(
            fs::read(application.join("result.json")).expect("private result"),
            b"{\"renderer\":42,\"sourceReadDenied\":true,\"sourceWriteDenied\":true}"
        );
        assert_eq!(
            fs::read_to_string(&sentinel).expect("unchanged user source"),
            PROBE_SENTINEL
        );
        assert!(
            fs::canonicalize(&stage)
                .unwrap()
                .starts_with(fs::canonicalize(&profile_dir).unwrap())
        );
        // Helper confirms zero owned descendants before returning the result.
        fs::remove_dir_all(&stage).expect("remove owned private compatibility stage");
        println!(
            "PRIVATE_STAGE_ELECTRON_PASS: real hidden renderer, sandbox/Node restrictions intact, private output, external source read/write denied, owned process tree drained; not a general GUI grant"
        );
    }

    fn copy_electron_runtime_for_probe(source: &Path, destination: &Path) {
        let mut pending = vec![(source.to_path_buf(), destination.to_path_buf(), 0_usize)];
        let mut files = 0_usize;
        let mut directories = 0_usize;
        let mut bytes = 0_u64;
        while let Some((from, to, depth)) = pending.pop() {
            assert!(depth <= 8);
            directories += 1;
            assert!(directories <= 256);
            let info = fs::symlink_metadata(&from).expect("runtime directory");
            assert!(info.is_dir() && !info.file_type().is_symlink());
            use std::os::windows::fs::MetadataExt;
            assert_eq!(
                info.file_attributes() & 0x400,
                0,
                "runtime reparse directory"
            );
            for item in fs::read_dir(&from).expect("runtime inventory") {
                let item = item.expect("runtime entry");
                let info = fs::symlink_metadata(item.path()).expect("runtime metadata");
                assert!(!info.file_type().is_symlink());
                assert_eq!(info.file_attributes() & 0x400, 0, "runtime reparse entry");
                let target = to.join(item.file_name());
                if info.is_dir() {
                    assert!(pending.len() + directories < 256);
                    fs::create_dir(&target).expect("new private runtime directory");
                    pending.push((item.path(), target, depth + 1));
                } else {
                    assert!(info.is_file() && info.len() <= 256 * 1024 * 1024);
                    files += 1;
                    bytes = bytes.checked_add(info.len()).unwrap();
                    assert!(files <= 256 && bytes <= 512 * 1024 * 1024);
                    let copied =
                        fs::copy(item.path(), &target).expect("copy selected optional runtime");
                    assert_eq!(copied, info.len(), "runtime changed during copy");
                }
            }
        }
        println!(
            "PRIVATE_ELECTRON_RUNTIME: {files} files / {bytes} bytes; original installation untouched"
        );
    }

    #[test]
    fn directory_metadata_grant_cannot_list_read_create_or_escape_and_pins_root() {
        let root = unique_temp_file();
        fs::create_dir(&root).expect("create unique metadata root");
        let sentinel = root.join("ungranted.txt");
        fs::write(&sentinel, PROBE_SENTINEL).expect("write ungranted file");
        let profile = AppContainerProfile::create_ephemeral().expect("ephemeral profile");
        let profile_dir = profile.profile_storage_path_for_tests();
        let probe = profile_dir.join("metadata-probe.exe");
        let _probe_cleanup = Cleanup(probe.clone());
        let _config_cleanup = Cleanup(profile_dir.join(PROBE_CONFIG));
        fs::copy(std::env::current_exe().unwrap(), &probe).expect("copy probe");
        assert_ne!(
            run_workspace_probe(&profile, &probe, &root, "metadata").exit_code,
            0
        );
        let grant = profile
            .grant_directory_metadata_within(&root, &root)
            .expect("metadata grant");
        let metadata = run_workspace_probe(&profile, &probe, &root, "metadata");
        assert_eq!(metadata.exit_code, 0, "{}", metadata.diagnostic());
        for mode in ["list", "create"] {
            assert_ne!(
                run_workspace_probe(&profile, &probe, &root, mode).exit_code,
                0
            );
        }
        assert_ne!(
            run_workspace_probe(&profile, &probe, &sentinel, "read").exit_code,
            0
        );
        assert!(!root.join("forbidden-create.txt").exists());
        assert!(
            profile
                .grant_directory_metadata_within(root.parent().unwrap(), &root)
                .is_err()
        );
        assert!(
            profile
                .grant_directory_metadata_within(&sentinel, &root)
                .is_err()
        );
        assert!(
            profile
                .grant_directory_metadata_within(Path::new("relative"), &root)
                .is_err()
        );
        let moved = root.with_extension("moved");
        assert!(
            fs::rename(&root, &moved).is_err(),
            "live metadata grant must pin root identity"
        );
        grant.revoke().expect("revoke metadata rights");
        assert_ne!(
            run_workspace_probe(&profile, &probe, &root, "metadata").exit_code,
            0
        );
        {
            let _drop_grant = profile
                .grant_directory_metadata_within(&root, &root)
                .unwrap();
            let metadata = run_workspace_probe(&profile, &probe, &root, "metadata");
            assert_eq!(metadata.exit_code, 0, "{}", metadata.diagnostic());
        }
        assert_ne!(
            run_workspace_probe(&profile, &probe, &root, "metadata").exit_code,
            0
        );
        let alias = root.with_extension("alias");
        if std::os::windows::fs::symlink_dir(&root, &alias).is_ok() {
            assert!(
                profile
                    .grant_directory_metadata_within(&alias, &root)
                    .is_err()
            );
            assert!(
                profile
                    .grant_directory_metadata_within(&root, &alias)
                    .is_err()
            );
            fs::remove_dir(&alias).expect("remove owned directory symlink only");
        }
        fs::rename(&root, &moved).expect("pins released after revoke");
        fs::rename(&moved, &root).expect("restore fixture name");
        fs::remove_file(&sentinel).expect("remove owned sentinel");
        fs::remove_dir(&root).expect("remove empty owned root");
    }

    #[test]
    fn ephemeral_read_grant_is_exact_read_only_and_revocable() {
        let sentinel = unique_temp_file();
        fs::write(&sentinel, PROBE_SENTINEL).expect("write sentinel");
        let _cleanup = Cleanup(sentinel.clone());
        let ungranted = unique_temp_file();
        fs::write(&ungranted, PROBE_SENTINEL).expect("write ungranted sentinel");
        let _ungranted_cleanup = Cleanup(ungranted.clone());

        assert_eq!(
            fs::read_to_string(&sentinel).expect("control read sentinel"),
            PROBE_SENTINEL
        );

        let profile = AppContainerProfile::create_ephemeral().expect("create ephemeral profile");
        let profile_dir = profile.profile_storage_path_for_tests();
        assert!(
            profile_dir.is_dir(),
            "AppContainer profile storage directory must exist: {}",
            profile_dir.display()
        );
        let probe_exe = profile_dir.join("optic-workspace-grant-probe.exe");
        let probe_config = profile_dir.join(PROBE_CONFIG);
        let _probe_cleanup = Cleanup(probe_exe.clone());
        let _config_cleanup = Cleanup(probe_config);
        fs::copy(
            std::env::current_exe().expect("current test executable"),
            &probe_exe,
        )
        .expect("copy workspace probe into AppContainer private storage");

        let before_grant = run_workspace_probe(&profile, &probe_exe, &sentinel, "read");
        assert_ne!(
            before_grant.exit_code,
            0,
            "profile without an ACL grant must not read the file: {}",
            before_grant.diagnostic()
        );

        let grant = profile
            .grant_file_read_within(&sentinel, sentinel.parent().expect("sentinel parent"))
            .expect("grant exact-file read within workspace root");
        let granted_read = run_workspace_probe(&profile, &probe_exe, &sentinel, "read");
        assert_eq!(
            granted_read.exit_code,
            0,
            "Package SID plus exact ACL grant must read the file: {}",
            granted_read.diagnostic()
        );
        let ungranted_read = run_workspace_probe(&profile, &probe_exe, &ungranted, "read");
        assert_ne!(
            ungranted_read.exit_code,
            0,
            "the profile must not read a second file without its own ACL grant: {}",
            ungranted_read.diagnostic()
        );

        let append_attempt = run_workspace_probe(&profile, &probe_exe, &sentinel, "append");
        assert_ne!(
            append_attempt.exit_code,
            0,
            "read-only grant must not permit file mutation: {}",
            append_attempt.diagnostic()
        );
        assert_eq!(
            fs::read_to_string(&sentinel).expect("read sentinel after write attempt"),
            PROBE_SENTINEL
        );

        grant.revoke().expect("revoke exact-file grant");
        let revoked_read = run_workspace_probe(&profile, &probe_exe, &sentinel, "read");
        assert_ne!(
            revoked_read.exit_code,
            0,
            "the same profile must lose access after its ACE is revoked: {}",
            revoked_read.diagnostic()
        );

        {
            let _drop_revoke = profile
                .grant_file_read_within(&sentinel, sentinel.parent().expect("sentinel parent"))
                .expect("grant exact-file read within workspace root for Drop cleanup");
            let live_read = run_workspace_probe(&profile, &probe_exe, &sentinel, "read");
            assert_eq!(
                live_read.exit_code,
                0,
                "Drop-cleanup grant must be usable while its guard is live: {}",
                live_read.diagnostic()
            );
        }
        let dropped_read = run_workspace_probe(&profile, &probe_exe, &sentinel, "read");
        assert_ne!(
            dropped_read.exit_code,
            0,
            "dropping the grant guard must revoke the Package SID ACE: {}",
            dropped_read.diagnostic()
        );
    }

    struct Cleanup(PathBuf);

    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
}
