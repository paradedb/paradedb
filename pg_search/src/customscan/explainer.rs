use pgrx::pg_sys;
use pgrx::pg_sys::AsPgCStr;
use std::ptr::NonNull;

pub struct Explainer {
    state: NonNull<pg_sys::ExplainState>,
}

impl Explainer {
    pub fn new(state: *mut pg_sys::ExplainState) -> Option<Self> {
        NonNull::new(state).map(|state| Self { state })
    }

    pub fn add_text(&mut self, key: &str, value: &str) {
        unsafe {
            pg_sys::ExplainPropertyText(key.as_pg_cstr(), value.as_pg_cstr(), self.state.as_ptr());
        }
    }

    pub fn add_integer(&mut self, key: &str, value: i64, unit: Option<&str>) {
        unsafe {
            pg_sys::ExplainPropertyInteger(
                key.as_pg_cstr(),
                unit.as_pg_cstr(),
                value,
                self.state.as_ptr(),
            );
        }
    }

    pub fn add_unsigned_integer(&mut self, key: &str, value: u64, unit: Option<&str>) {
        unsafe {
            pg_sys::ExplainPropertyUInteger(
                key.as_pg_cstr(),
                unit.as_pg_cstr(),
                value,
                self.state.as_ptr(),
            );
        }
    }

    pub fn add_float(&mut self, key: &str, value: f64, unit: Option<&str>, ndigits: i32) {
        unsafe {
            pg_sys::ExplainPropertyFloat(
                key.as_pg_cstr(),
                unit.as_pg_cstr(),
                value,
                ndigits,
                self.state.as_ptr(),
            );
        }
    }

    pub fn add_bool(&mut self, key: &str, value: bool) {
        unsafe {
            pg_sys::ExplainPropertyBool(key.as_pg_cstr(), value, self.state.as_ptr());
        }
    }

    pub fn add_list(&mut self, key: &str, values: &mut pgrx::list::List<*mut std::ffi::c_char>) {
        unsafe {
            pg_sys::ExplainPropertyList(key.as_pg_cstr(), values.as_mut_ptr(), self.state.as_ptr())
        }
    }
}
