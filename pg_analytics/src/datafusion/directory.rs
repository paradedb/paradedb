use crate::errors::ParadeError;
use pgrx::*;
use std::ffi::{CStr, CString};
use std::fs;
use std::path::PathBuf;

pub static PARADE_DIRECTORY: &str = "deltalake";

pub struct ParadeDirectory;

impl ParadeDirectory {
    pub fn delta_path() -> Result<PathBuf, ParadeError> {
        let data_dir = ParadeDirectory::root_path()?;
        let delta_dir = data_dir.join(PARADE_DIRECTORY);

        Ok(delta_dir)
    }

    pub fn schema_path(schema_oid: pg_sys::Oid) -> Result<PathBuf, ParadeError> {
        let delta_dir = Self::delta_path()?;
        let schema_dir = delta_dir.join(schema_oid.as_u32().to_string());

        Ok(schema_dir)
    }

    pub fn table_path(
        schema_oid: pg_sys::Oid,
        table_oid: pg_sys::Oid,
    ) -> Result<PathBuf, ParadeError> {
        let schema_dir = ParadeDirectory::schema_path(schema_oid)?;
        let table_dir = schema_dir.join(table_oid.as_u32().to_string());

        Ok(table_dir)
    }

    pub fn create_delta_path() -> Result<(), ParadeError> {
        let delta_dir = Self::delta_path()?;
        if !delta_dir.exists() {
            fs::create_dir_all(&delta_dir)?;
        }

        Ok(())
    }

    pub fn create_schema_path(schema_oid: pg_sys::Oid) -> Result<(), ParadeError> {
        let schema_dir = Self::schema_path(schema_oid)?;
        if !schema_dir.exists() {
            fs::create_dir_all(&schema_dir)?;
        }

        Ok(())
    }

    fn root_path() -> Result<PathBuf, ParadeError> {
        let option_name = CString::new("data_directory")?;
        let data_dir_str = unsafe {
            CStr::from_ptr(pg_sys::GetConfigOptionByName(
                option_name.as_ptr(),
                std::ptr::null_mut(),
                true,
            ))
            .to_str()?
        };

        Ok(PathBuf::from(data_dir_str))
    }
}
