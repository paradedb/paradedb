use pgrx::*;
use std::ffi::{CStr, CString};

use crate::errors::ParadeError;

const PARADE_DIRECTORY: &str = "paradedb";

pub struct ParquetDirectory;

impl ParquetDirectory {
    pub fn schema_path() -> Result<String, ParadeError> {
        let data_dir = ParquetDirectory::data_directory()?;
        let schema_dir = format!("{}/{}", data_dir, PARADE_DIRECTORY);

        Ok(schema_dir)
    }

    pub fn table_path(table_name: &str) -> Result<String, ParadeError> {
        let data_dir = ParquetDirectory::data_directory()?;
        let table_dir = format!("{}/{}/{}", data_dir, PARADE_DIRECTORY, table_name);

        Ok(table_dir)
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
