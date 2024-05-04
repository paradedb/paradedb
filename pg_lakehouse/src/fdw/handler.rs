use pgrx::*;

#[derive(PartialEq)]
pub enum FdwHandler {
    S3,
    LocalFile,
    Other,
}

impl From<pg_sys::Oid> for FdwHandler {
    fn from(oid: pg_sys::Oid) -> Self {
        let fdw = unsafe { pg_sys::GetForeignDataWrapper(oid) };
        let handler_oid = unsafe { (*fdw).fdwhandler };
        let proc_tuple = unsafe {
            pg_sys::SearchSysCache1(
                pg_sys::SysCacheIdentifier_PROCOID as i32,
                handler_oid.into_datum().unwrap(),
            )
        };
        let pg_proc = unsafe { pg_sys::GETSTRUCT(proc_tuple) as pg_sys::Form_pg_proc };
        let handler_name = unsafe { name_data_to_str(&(*pg_proc).proname) };
        unsafe { pg_sys::ReleaseSysCache(proc_tuple) };

        match handler_name {
            "s3_fdw_handler" => Self::S3,
            "local_file_fdw_handler" => Self::LocalFile,
            _ => Self::Other,
        }
    }
}
