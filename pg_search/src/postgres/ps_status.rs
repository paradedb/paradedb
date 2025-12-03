use pgrx::pg_sys;
use std::ffi::CStr;
use std::os::raw::c_char;

pub const INDEXING: &CStr = c"indexing";
pub const MERGING: &CStr = c"merging";
pub const COMMITTING: &CStr = c"committing";
pub const GARBAGE_COLLECTING: &CStr = c"gc-ing";
pub const FINALIZING: &CStr = c"finalizing";

pub unsafe fn set_ps_display_suffix(suffix: *const c_char) {
    #[cfg(any(feature = "pg14", feature = "pg15"))]
    pg_sys::set_ps_display(suffix);

    #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
    pg_sys::set_ps_display_suffix(suffix);
}

pub unsafe fn set_ps_display_remove_suffix() {
    #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
    pg_sys::set_ps_display_remove_suffix();
}
