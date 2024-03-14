use pgrx::*;

#[pg_guard]
pub extern "C" fn amvalidate(_opclassoid: pg_sys::Oid) -> bool {
    true
}
