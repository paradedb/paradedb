use pgrx::pg_sys;
use std::ffi::CStr;
use std::os::raw::c_char;

pub const INDEXING: &CStr = c"indexing";
pub const MERGING: &CStr = c"merging";

pub unsafe fn set_ps_display_suffix(suffix: *const c_char) {
    #[cfg(any(feature = "pg16", feature = "pg17"))]
    pg_sys::ffi::pg_guard_ffi_boundary(|| {
        extern "C-unwind" {
            pub fn set_ps_display_suffix(suffix: *const c_char);
        }
        set_ps_display_suffix(suffix);
    });

    #[cfg(any(feature = "pg14", feature = "pg15"))]
    pg_sys::ffi::pg_guard_ffi_boundary(|| {
        extern "C-unwind" {
            pub fn set_ps_display(suffix: *const c_char);
        }
        set_ps_display(suffix);
    });
}

pub unsafe fn set_ps_display_remove_suffix() {
    #[cfg(any(feature = "pg16", feature = "pg17"))]
    pg_sys::ffi::pg_guard_ffi_boundary(|| {
        extern "C-unwind" {
            pub fn set_ps_display_remove_suffix();
        }
        set_ps_display_remove_suffix();
    });
}
