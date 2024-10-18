use pgrx::{pg_guard, pg_sys};
use std::ptr::addr_of_mut;

#[derive(Debug)]
#[repr(transparent)]
struct Spinlock(pg_sys::slock_t);

impl Spinlock {
    #[inline(always)]
    pub fn acquire(&mut self) -> impl Drop {
        AcquiredSpinLock::new(self)
    }
}

#[repr(transparent)]
struct AcquiredSpinLock(*mut pg_sys::slock_t);

impl AcquiredSpinLock {
    fn new(lock: &mut Spinlock) -> Self {
        unsafe {
            let addr = addr_of_mut!(lock.0);
            pg_sys::SpinLockAcquire(addr);
            Self(addr)
        }
    }
}

impl Drop for AcquiredSpinLock {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            pg_sys::SpinLockRelease(self.0);
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Bm25ParallelScanState {
    mutex: Spinlock,
    remaining_segments: u32,
}

impl Bm25ParallelScanState {
    #[inline(always)]
    pub fn lock(&mut self) -> impl Drop {
        self.mutex.acquire()
    }
}

#[pg_guard]
pub unsafe extern "C" fn aminitparallelscan(target: *mut ::core::ffi::c_void) {
    let state = target.cast::<Bm25ParallelScanState>();
    pg_sys::SpinLockInit(addr_of_mut!((*state).mutex.0));
}

#[pg_guard]
pub unsafe extern "C" fn amparallelrescan(_scan: pg_sys::IndexScanDesc) {}

#[pg_guard]
pub unsafe extern "C" fn amestimateparallelscan() -> pg_sys::Size {
    size_of::<Bm25ParallelScanState>()
}

unsafe fn bm25_shared_state(
    scan: &pg_sys::IndexScanDescData,
) -> Option<&mut Bm25ParallelScanState> {
    if scan.parallel_scan.is_null() {
        None
    } else {
        scan.parallel_scan
            .cast::<std::ffi::c_void>()
            .add((*scan.parallel_scan).ps_offset)
            .cast::<Bm25ParallelScanState>()
            .as_mut()
    }
}

pub unsafe fn maybe_init_parallel_scan(
    scan: pg_sys::IndexScanDesc,
    searcher: &tantivy::Searcher,
) -> Option<i32> {
    let state = get_bm25_scan_state(&scan)?;

    let _mutex = state.lock();
    if pg_sys::ParallelWorkerNumber == -1 {
        // ParallelWorkerNumber -1 is the main backend, which is where we'll set up
        // our shared memory information
        state.remaining_segments = searcher
            .segment_readers()
            .len()
            .try_into()
            .expect("should not have more than u32 index segments");
    }
    Some(pg_sys::ParallelWorkerNumber)
}

pub unsafe fn maybe_claim_segment(scan: pg_sys::IndexScanDesc) -> Option<tantivy::SegmentOrdinal> {
    let state = get_bm25_scan_state(&scan)?;

    let _mutex = state.lock();
    if state.remaining_segments == 0 {
        // no more to check out
        None
    } else {
        // check one out
        state.remaining_segments -= 1;
        Some(state.remaining_segments)
    }
}

unsafe fn get_bm25_scan_state(scan: &pg_sys::IndexScanDesc) -> Option<&mut Bm25ParallelScanState> {
    assert!(!scan.is_null());
    let scan = scan.as_mut().unwrap_unchecked();
    let state = bm25_shared_state(scan)?;
    Some(state)
}
