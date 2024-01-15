use pgrx::*;
use std::ffi::{CStr, CString};
use std::fs;
use std::path::Path;

use crate::errors::ParadeError;

const PARADE_DIRECTORY: &str = "paradedb";

pub struct ParadeDirectory;

impl ParadeDirectory {
    pub fn schema_path() -> Result<String, ParadeError> {
        let data_dir = ParadeDirectory::data_directory()?;
        let schema_dir = format!("{}/{}", data_dir, PARADE_DIRECTORY);

        Ok(schema_dir)
    }

    pub fn table_path(table_name: &str) -> Result<String, ParadeError> {
        let data_dir = ParadeDirectory::data_directory()?;
        let table_dir = format!("{}/{}/{}", data_dir, PARADE_DIRECTORY, table_name);

        Ok(table_dir)
    }

    pub fn create_schema_path() -> Result<(), ParadeError> {
        let schema_path = ParadeDirectory::schema_path()?;
        if !Path::new(&schema_path).exists() {
            fs::create_dir_all(&schema_path)?;
        }
        Ok(())
    }

    fn data_directory() -> Result<String, ParadeError> {
        let option_name = CString::new("data_directory")?;
        let data_dir = String::from_utf8(
            unsafe {
                CStr::from_ptr(pg_sys::GetConfigOptionByName(
                    option_name.as_ptr(),
                    std::ptr::null_mut(),
                    true,
                ))
            }
            .to_bytes()
            .to_vec(),
        )?;

        Ok(data_dir)
    }
}
