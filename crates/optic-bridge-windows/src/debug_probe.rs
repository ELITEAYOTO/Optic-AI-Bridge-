//! Test-only debugger for a freshly created, owned public compatibility child.
//! No attach API or arbitrary PID/process-memory surface is exposed.
use crate::SuspendedAppContainerProcess;
use std::{
    io::{Error, Result},
    os::windows::io::AsRawHandle,
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{
        CloseHandle, DBG_CONTINUE, DBG_EXCEPTION_NOT_HANDLED, EXCEPTION_BREAKPOINT, HANDLE,
    },
    System::{
        Diagnostics::Debug::{
            CREATE_PROCESS_DEBUG_EVENT, ContinueDebugEvent, DEBUG_EVENT, DebugActiveProcessStop,
            EXCEPTION_DEBUG_EVENT, EXIT_PROCESS_DEBUG_EVENT, LOAD_DLL_DEBUG_EVENT,
            OUTPUT_DEBUG_STRING_EVENT, ReadProcessMemory, WaitForDebugEventEx,
        },
        Threading::GetProcessId,
    },
};

#[cfg(target_arch = "x86_64")]
#[repr(C, align(16))]
struct AlignedIntegerContext(windows::Win32::System::Diagnostics::Debug::CONTEXT);

#[cfg(target_arch = "x86_64")]
#[test]
fn integer_context_buffer_is_natively_aligned() {
    let context = AlignedIntegerContext(Default::default());
    assert!(align_of::<AlignedIntegerContext>() >= 16);
    assert_eq!((&context.0 as *const _ as usize) % 16, 0);
}

#[cfg(target_arch = "x86_64")]
fn primary_integer_result(thread: HANDLE) -> Result<u32> {
    use windows::Win32::System::Diagnostics::Debug::{
        CONTEXT, CONTEXT_INTEGER_AMD64, GetThreadContext,
    };
    let mut context = AlignedIntegerContext(CONTEXT {
        ContextFlags: CONTEXT_INTEGER_AMD64,
        ..CONTEXT::default()
    });
    // SAFETY: this is the initial-thread handle from CREATE_PROCESS_DEBUG_EVENT
    // for our freshly created child, still stopped at its own exception event.
    // Never opens a PID/thread chosen by a caller and never sets context.
    unsafe { GetThreadContext(thread, &mut context.0) }.map_err(Error::other)?;
    Ok(context.0.Rax as u32)
}

#[cfg(not(target_arch = "x86_64"))]
fn primary_integer_result(_thread: HANDLE) -> Result<u32> {
    Err(Error::other("owned context architecture unsupported"))
}

pub(crate) fn wait_owned_debug_probe(
    child: &mut SuspendedAppContainerProcess,
    timeout_ms: u32,
) -> Result<u32> {
    let handle = HANDLE(child.borrowed_process_for_tests().as_raw_handle());
    // SAFETY: handle is borrowed from this uniquely owned child, never a PID lookup.
    let pid = unsafe { GetProcessId(handle) };
    if pid == 0 {
        return Err(Error::other("owned debug process identity unavailable"));
    }
    struct DetachOwnedProbe(u32);
    impl Drop for DetachOwnedProbe {
        fn drop(&mut self) {
            // SAFETY: this PID was obtained only from our live owned child,
            // created with DEBUG_ONLY_THIS_PROCESS on this same thread. Detach
            // also lets the owned Job Object confirm exit on an error path.
            let _ = unsafe { DebugActiveProcessStop(self.0) };
        }
    }
    let _detach = DetachOwnedProbe(pid);
    let until = Instant::now() + Duration::from_millis(u64::from(timeout_ms));
    let mut initial_breakpoint = false;
    let mut image_base = 0_usize;
    let mut primary_thread = HANDLE::default();
    let mut primary_thread_id = 0;
    let mut remaining_log = 32 * 1024_usize;
    for _ in 0..4096 {
        if Instant::now() >= until {
            return Err(Error::other("owned debug probe deadline"));
        }
        let mut event = DEBUG_EVENT::default();
        // SAFETY: event is writable output; debugger runs on the creation thread.
        if let Err(error) = unsafe { WaitForDebugEventEx(&mut event, 100) } {
            if error.code() == windows::core::HRESULT::from_win32(121) {
                continue;
            }
            return Err(Error::other(error));
        }
        if event.dwProcessId != pid {
            return Err(Error::other("unexpected debug process identity"));
        }
        let mut status = DBG_CONTINUE;
        let mut exited = false;
        let mut message = None;
        // SAFETY: each union variant is read only after checking its OS event tag.
        unsafe {
            match event.dwDebugEventCode {
                CREATE_PROCESS_DEBUG_EVENT => {
                    let info = event.u.CreateProcessInfo;
                    image_base = info.lpBaseOfImage as usize;
                    primary_thread = info.hThread;
                    primary_thread_id = event.dwThreadId;
                    if !info.hFile.is_invalid() {
                        let _ = CloseHandle(info.hFile);
                    }
                }
                LOAD_DLL_DEBUG_EVENT => {
                    let info = event.u.LoadDll;
                    if !info.hFile.is_invalid() {
                        let _ = CloseHandle(info.hFile);
                    }
                }
                OUTPUT_DEBUG_STRING_EVENT => {
                    let info = event.u.DebugString;
                    let units = usize::from(info.nDebugStringLength);
                    let size = (units * if info.fUnicode == 0 { 1 } else { 2 })
                        .min(8192)
                        .min(remaining_log);
                    if size != 0 {
                        let mut bytes = vec![0_u8; size];
                        let mut read = 0_usize;
                        // Only this child is read; OS-provided size/address are
                        // untrusted and capped before allocation or memory I/O.
                        if ReadProcessMemory(
                            handle,
                            info.lpDebugStringData.0.cast(),
                            bytes.as_mut_ptr().cast(),
                            size,
                            Some(&mut read),
                        )
                        .is_ok()
                        {
                            bytes.truncate(read.min(size));
                            let text = if info.fUnicode == 0 {
                                String::from_utf8_lossy(&bytes).into_owned()
                            } else {
                                String::from_utf16_lossy(
                                    &bytes
                                        .as_chunks::<2>()
                                        .0
                                        .iter()
                                        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                                        .collect::<Vec<_>>(),
                                )
                            };
                            message = Some(format!("string={:?}", text.trim_end_matches('\0')));
                        }
                    }
                }
                EXCEPTION_DEBUG_EVENT => {
                    let info = event.u.Exception;
                    let code = info.ExceptionRecord.ExceptionCode;
                    let primary_eax = if initial_breakpoint
                        && code == EXCEPTION_BREAKPOINT
                        && event.dwThreadId == primary_thread_id
                        && !primary_thread.is_invalid()
                    {
                        Some(primary_integer_result(primary_thread))
                    } else {
                        None
                    };
                    message = Some(format!(
                        "exception={:08x} first_chance={} offset_from_main_image={:?} primary_thread_match={} primary_eax={primary_eax:?}",
                        code.0 as u32,
                        info.dwFirstChance,
                        (info.ExceptionRecord.ExceptionAddress as usize).checked_sub(image_base),
                        event.dwThreadId == primary_thread_id
                    ));
                    if code == EXCEPTION_BREAKPOINT && !initial_breakpoint {
                        initial_breakpoint = true;
                    } else {
                        status = DBG_EXCEPTION_NOT_HANDLED;
                    }
                }
                EXIT_PROCESS_DEBUG_EVENT => {
                    exited = true;
                }
                _ => {}
            }
        }
        if let Some(message) = message
            && message.len() <= remaining_log
        {
            remaining_log -= message.len();
            eprintln!("PRIVATE_OWNED_DEBUG: {message}");
        }
        // SAFETY: resumes exactly the event supplied by Windows for our child.
        unsafe { ContinueDebugEvent(event.dwProcessId, event.dwThreadId, status) }
            .map_err(Error::other)?;
        if exited {
            return child.wait_exit(1000);
        }
    }
    Err(Error::other("owned debug probe event ceiling"))
}
