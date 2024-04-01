use pgrx::*;
use std::ffi::{CStr, CString};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

use super::session::Session;

pub static PARADE_DIRECTORY: &str = "deltalake";

pub struct ParadeDirectory;

impl ParadeDirectory {
    pub fn catalog_path(catalog_oid: pg_sys::Oid) -> Result<PathBuf, DirectoryError> {
        let delta_dir = Self::root_path()?;
        let catalog_dir = delta_dir.join(catalog_oid.as_u32().to_string());

        Ok(catalog_dir)
    }

    pub fn schema_path(
        catalog_oid: pg_sys::Oid,
        schema_oid: pg_sys::Oid,
    ) -> Result<PathBuf, DirectoryError> {
        let delta_dir = Self::catalog_path(catalog_oid)?;
        let schema_dir = delta_dir.join(schema_oid.as_u32().to_string());

        Ok(schema_dir)
    }

    pub fn table_path_from_name(
        schema_name: &str,
        table_name: &str,
    ) -> Result<PathBuf, DirectoryError> {
        let pg_relation =
            match unsafe { PgRelation::open_with_name(&format!("{}.{}", schema_name, table_name)) }
            {
                Ok(relation) => relation,
                Err(_) => {
                    return Err(DirectoryError::RelationNotFound(
                        schema_name.to_string(),
                        table_name.to_string(),
                    ))
                }
            };

        Self::table_path_from_oid(pg_relation.namespace_oid(), pg_relation.oid())
    }

    pub fn table_path_from_oid(
        schema_oid: pg_sys::Oid,
        table_oid: pg_sys::Oid,
    ) -> Result<PathBuf, DirectoryError> {
        let schema_dir = ParadeDirectory::schema_path(Session::catalog_oid(), schema_oid)?;
        let table_dir = schema_dir.join(table_oid.as_u32().to_string());

        Ok(table_dir)
    }

    pub fn create_catalog_path(catalog_oid: pg_sys::Oid) -> Result<(), DirectoryError> {
        let catalog_dir = Self::catalog_path(catalog_oid)?;
        if !catalog_dir.exists() {
            fs::create_dir_all(&catalog_dir)?;
        }

        Ok(())
    }

    pub fn create_schema_path(
        catalog_oid: pg_sys::Oid,
        schema_oid: pg_sys::Oid,
    ) -> Result<(), DirectoryError> {
        let schema_dir = Self::schema_path(catalog_oid, schema_oid)?;
        if !schema_dir.exists() {
            fs::create_dir_all(&schema_dir)?;
        }

        Ok(())
    }

    fn root_path() -> Result<PathBuf, DirectoryError> {
        let option_name = CString::new("data_directory")?;
        let data_dir_str = unsafe {
            CStr::from_ptr(pg_sys::GetConfigOptionByName(
                option_name.as_ptr(),
                std::ptr::null_mut(),
                true,
            ))
            .to_str()?
        };

        Ok(PathBuf::from(data_dir_str).join(PARADE_DIRECTORY))
    }
}

#[derive(Error, Debug)]
pub enum DirectoryError {
    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    NulError(#[from] std::ffi::NulError),

    #[error(transparent)]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Could not open relation for {0}.{1}")]
    RelationNotFound(String, String),
}
