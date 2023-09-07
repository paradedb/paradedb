use pgrx::*;

// For now, we aren't supporting any options
#[pg_guard]
pub unsafe extern "C" fn amoptions(
    _reloptions: pg_sys::Datum,
    _validate: bool,
) -> *mut pg_sys::bytea {
    std::ptr::null_mut()
}
