use once_cell::sync::Lazy;
use pgrx::pg_sys::AsPgCStr;
use pgrx::*;

pub static RM_ANALYTICS_ID: u8 = 135;

pub static mut CUSTOM_RMGR: Lazy<pg_sys::RmgrData> = Lazy::new(|| pg_sys::RmgrData {
    rm_name: "pg_analytics".as_pg_cstr(),
    rm_redo: None,
    rm_desc: None,
    rm_identify: None,
    rm_startup: None,
    rm_cleanup: None,
    rm_mask: None,
    rm_decode: None,
});
