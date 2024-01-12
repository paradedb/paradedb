use pgrx::*;
use std::ffi::{CStr, CString, NulError};
use std::string::FromUtf8Error;

const PARADE_DIRECTORY: &str = "paradedb";

pub struct ParquetDirectory;

impl ParquetDirectory {
    pub fn schema_path() -> Result<String, String> {
        let data_dir = ParquetDirectory::data_directory()?;
        let schema_dir = format!("{}/{}", data_dir, PARADE_DIRECTORY);

        Ok(schema_dir)
    }

    pub fn table_path(table_name: &str) -> Result<String, String> {
        let data_dir = ParquetDirectory::data_directory()?;
        let table_dir = format!("{}/{}/{}", data_dir, PARADE_DIRECTORY, table_name);

        Ok(table_dir)
    }

    fn data_directory() -> Result<String, String> {
        let option_name = CString::new("data_directory")
            .map_err(|e: NulError| format!("Failed to create CString: {}", e))?;
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
        )
        .map_err(|e: FromUtf8Error| format!("Failed to convert C string to Rust string: {}", e))?;

        Ok(data_dir)
    }
}
