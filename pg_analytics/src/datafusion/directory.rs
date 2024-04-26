use pgrx::*;
use std::ffi::{CStr, CString};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

use super::session::Session;

pub static PARADE_DIRECTORY: &str = "deltalake";

pub struct ParadeDirectory;

impl ParadeDirectory {
    pub fn catalog_path(
        tablespace_oid: pg_sys::Oid,
        catalog_oid: pg_sys::Oid,
    ) -> Result<PathBuf, DirectoryError> {
        let delta_dir = Self::root_path(tablespace_oid)?;
        let catalog_dir = delta_dir.join(catalog_oid.as_u32().to_string());

        Ok(catalog_dir)
    }

    pub fn schema_path(
        tablespace_oid: pg_sys::Oid,
        catalog_oid: pg_sys::Oid,
        schema_oid: pg_sys::Oid,
    ) -> Result<PathBuf, DirectoryError> {
        let delta_dir = Self::catalog_path(tablespace_oid, catalog_oid)?;
        let schema_dir = delta_dir.join(schema_oid.as_u32().to_string());

        Ok(schema_dir)
    }

    pub fn table_path_from_name(
        schema_name: &str,
        table_name: &str,
    ) -> Result<PathBuf, DirectoryError> {
        let pg_relation = match unsafe {
            PgRelation::open_with_name(format!("{}.{}", schema_name, table_name).as_str())
        } {
            Ok(relation) => relation,
            Err(_) => {
                return Err(DirectoryError::RelationNotFound(
                    schema_name.to_string(),
                    table_name.to_string(),
                ))
            }
        };

        let tablespace_oid = unsafe { pg_sys::get_rel_tablespace(pg_relation.oid()) };
        Self::table_path_from_oid(
            tablespace_oid,
            pg_relation.namespace_oid(),
            pg_relation.oid(),
        )
    }

    pub fn table_path_from_oid(
        tablespace_oid: pg_sys::Oid,
        schema_oid: pg_sys::Oid,
        table_oid: pg_sys::Oid,
    ) -> Result<PathBuf, DirectoryError> {
        let schema_dir =
            ParadeDirectory::schema_path(tablespace_oid, Session::catalog_oid(), schema_oid)?;
        let table_dir = schema_dir.join(table_oid.as_u32().to_string());

        Ok(table_dir)
    }

    pub fn create_schema_path(
        tablespace_oid: pg_sys::Oid,
        catalog_oid: pg_sys::Oid,
        schema_oid: pg_sys::Oid,
    ) -> Result<(), DirectoryError> {
        let schema_dir = Self::schema_path(tablespace_oid, catalog_oid, schema_oid)?;
        if !schema_dir.exists() {
            fs::create_dir_all(&schema_dir)?;
        }

        Ok(())
    }

    fn root_path(tablespace_oid: pg_sys::Oid) -> Result<PathBuf, DirectoryError> {
        let root_dir = unsafe {
            match tablespace_oid == pg_sys::InvalidOid {
                true => {
                    let data_directory_opt = pg_sys::GetConfigOptionByName(
                        CString::new("data_directory")?.as_ptr(),
                        std::ptr::null_mut(),
                        true,
                    );
                    let ret_string = CStr::from_ptr(data_directory_opt).to_str()?.to_string();

                    pg_sys::pfree(data_directory_opt as *mut std::ffi::c_void);

                    ret_string
                }
                false => direct_function_call::<String>(
                    pg_sys::pg_tablespace_location,
                    &[Some(pg_sys::Datum::from(tablespace_oid))],
                )
                .ok_or(DirectoryError::TableSpaceNotFound)?,
            }
        };

        Ok(PathBuf::from(root_dir).join(PARADE_DIRECTORY))
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

    #[error("Could not get default tablespace location")]
    TableSpaceNotFound,

    #[error("Could not open relation for {0}.{1}")]
    RelationNotFound(String, String),
}
