//! Research adapter: actual Win32 observation outside locks; existing admission
//! formula consumes that observation under JobStore. No fabricated memory values.
use super::*;
use std::{cell::Cell, marker::PhantomData, rc::Rc};

thread_local! {
    static SAMPLE: Cell<Option<HostMemorySnapshot>> = const { Cell::new(None) };
}

#[derive(Debug)]
struct ScopedObservationProvider;
impl HostMemoryProvider for ScopedObservationProvider {
    fn snapshot(&self) -> io::Result<HostMemorySnapshot> {
        SAMPLE
            .with(Cell::get)
            .ok_or_else(|| io::Error::other("no scoped actual host-memory observation"))
    }
}

pub(super) struct SampleScope {
    _not_send: PhantomData<Rc<()>>,
}
impl SampleScope {
    pub fn capture(manager: &ProcessManager) -> io::Result<Self> {
        if SAMPLE.with(Cell::get).is_some() {
            return Err(io::Error::other("nested memory scope"));
        }
        // Reject capture under this fixture's JobStore lock before ANY Win32.
        // This is not a worst-case timing claim about OS scheduler progress.
        let unlocked = manager
            .jobs
            .try_lock()
            .map_err(|_| io::Error::other("memory capture while JobStore busy"))?;
        drop(unlocked);
        let observed = WindowsHostMemoryProvider.snapshot()?;
        SAMPLE.with(|sample| sample.set(Some(observed)));
        Ok(Self {
            _not_send: PhantomData,
        })
    }
}
impl Drop for SampleScope {
    fn drop(&mut self) {
        SAMPLE.with(|sample| sample.set(None));
    }
}

pub(super) fn manager(path: &Path, limits: HardLimits) -> Arc<ProcessManager> {
    let mut manager = ProcessManager::new(path, limits, Vec::new()).unwrap();
    manager.host_memory_provider = Arc::new(ScopedObservationProvider);
    Arc::new(manager)
}

#[test]
fn real_sample_is_required_scoped_and_capture_is_rejected_under_store_lock() {
    let root = env::temp_dir(); // read-only constructor target; no process/workspace writes
    let manager = manager(&root, HardLimits::default());
    assert!(manager.host_memory_provider.snapshot().is_err());
    {
        let guard = manager.jobs.lock().unwrap();
        assert!(SampleScope::capture(&manager).is_err());
        drop(guard);
    }
    {
        let _scope = SampleScope::capture(&manager).unwrap();
        let captured = manager.host_memory_provider.snapshot().unwrap();
        let _guard = manager.jobs.lock().unwrap();
        let under_lock = manager.host_memory_provider.snapshot().unwrap();
        assert_eq!(
            captured.total_physical_bytes,
            under_lock.total_physical_bytes
        );
        assert_eq!(
            captured.available_physical_bytes,
            under_lock.available_physical_bytes
        );
        assert!(captured.total_physical_bytes > 0);
    }
    assert!(manager.host_memory_provider.snapshot().is_err());
}
