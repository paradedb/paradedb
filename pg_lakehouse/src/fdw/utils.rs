use pgrx::pg_sys::AsPgCStr;
use pgrx::*;
use supabase_wrappers::prelude::*;
use thiserror::Error;

use crate::datafusion::context::ContextError;

use super::base::*;
use super::gcs::GcsFdw;
use super::handler::*;
use super::local::LocalFileFdw;
use super::s3::S3Fdw;

pub fn register_object_store(server: &str) -> Result<(), StoreUtilsError> {
    let foreign_server = unsafe { pg_sys::GetForeignServerByName(server.as_pg_cstr(), true) };

    if foreign_server.is_null() {
        return Err(StoreUtilsError::ForeignServerNotFound(server.to_string()));
    }

    let server_options = unsafe { options_to_hashmap((*foreign_server).options) }?;
    let user_mapping_options = unsafe { user_mapping_options(foreign_server) };
    let fdw_handler = unsafe { FdwHandler::from(foreign_server) };

    match fdw_handler {
        FdwHandler::S3 => {
            S3Fdw::register_object_store(server_options, user_mapping_options)?;
        }
        FdwHandler::LocalFile => {
            LocalFileFdw::register_object_store(server_options, user_mapping_options)?;
        }
        FdwHandler::Gcs => {
            GcsFdw::register_object_store(server_options, user_mapping_options)?;
        }
        _ => {
            return Err(StoreUtilsError::InvalidServerName(server.to_string()));
        }
    }

    Ok(())
}

#[derive(Error, Debug)]
pub enum StoreUtilsError {
    #[error(transparent)]
    ContextError(#[from] ContextError),

    #[error(transparent)]
    OptionsError(#[from] supabase_wrappers::prelude::OptionsError),

    #[error("No foreign server with name {0} was found")]
    ForeignServerNotFound(String),

    #[error("Server {0} was not created by this extension")]
    InvalidServerName(String),
}
