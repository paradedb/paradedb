use crate::postgres::scan::PgSearchScanState;
use pgrx::{pg_guard, pg_sys};
use std::ptr::addr_of_mut;

#[derive(Debug)]
#[repr(C)]
pub struct PgSearchParallelScanState {
    pub mutex: pg_sys::slock_t,
    pub worker_number: usize,
}

#[pg_guard]
pub unsafe extern "C" fn aminitparallelscan(target: *mut ::core::ffi::c_void) {
    let state = target.cast::<PgSearchParallelScanState>();
    pg_sys::SpinLockInit(addr_of_mut!((*state).mutex));
    pgrx::warning!("aminitparallelscan: {:?}", *state);
}

#[pg_guard]
pub unsafe extern "C" fn amparallelrescan(scan: pg_sys::IndexScanDesc) {
    pgrx::warning!("amparallelrescan");
}

#[pg_guard]
pub unsafe extern "C" fn amestimateparallelscan() -> pg_sys::Size {
    pgrx::warning!("amestimateparallelscan");
    size_of::<PgSearchParallelScanState>()
}
