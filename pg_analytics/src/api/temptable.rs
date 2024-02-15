use async_std::task;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::catalog::CatalogProvider;
use pgrx::*;
use std::ffi::CStr;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::schema::ObjectStoreSchemaProvider;
use crate::errors::{NotFound, ParadeError};

const DUMMY_TABLE_NAME: &str = "paradedb_dummy_foreign_parquet_table";

extension_sql!(
    r#"
    CREATE OR REPLACE PROCEDURE create_foreign_parquet_table(
        table_name TEXT,
        foreign_table_name TEXT,
        foreign_nickname TEXT
    ) 
    LANGUAGE C AS 'MODULE_PATHNAME', 'create_foreign_parquet_table';
    "#,
    name = "create_foreign_parquet_table"
);
#[pg_guard]
#[no_mangle]
pub extern "C" fn create_foreign_parquet_table(fcinfo: pg_sys::FunctionCallInfo) {
    create_foreign_parquet_table_impl(fcinfo).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

fn create_foreign_parquet_table_impl(fcinfo: pg_sys::FunctionCallInfo) -> Result<(), ParadeError> {
    let table_name: String = unsafe { fcinfo::pg_getarg(fcinfo, 0).unwrap() };
    let foreign_table_name: String = unsafe { fcinfo::pg_getarg(fcinfo, 1).unwrap() };
    let foreign_nickname: String = unsafe { fcinfo::pg_getarg(fcinfo, 2).unwrap() };

    let temp_schema_oid = unsafe {
        match direct_function_call::<pg_sys::Oid>(pg_sys::pg_my_temp_schema, &[]) {
            Some(pg_sys::InvalidOid) => {
                spi::Spi::run(&format!("CREATE TEMP TABLE {} (a int)", DUMMY_TABLE_NAME))?;

                match direct_function_call::<pg_sys::Oid>(pg_sys::pg_my_temp_schema, &[]) {
                    Some(pg_sys::InvalidOid) => return Err(NotFound::TempSchemaOid.into()),
                    Some(oid) => oid,
                    _ => return Err(NotFound::TempSchemaOid.into()),
                }
            }
            Some(oid) => oid,
            _ => return Err(NotFound::TempSchemaOid.into()),
        }
    };

    let temp_schema_name =
        unsafe { CStr::from_ptr(pg_sys::get_namespace_name(temp_schema_oid)).to_str()? };

    info!("got temp schema name: {}", temp_schema_name);

    let listing_table =
        DatafusionContext::with_object_store_schema_provider(&foreign_nickname, |provider| {
            task::block_on(provider.table(&foreign_table_name))
                .ok_or(NotFound::Table(foreign_table_name).into())
        })?;

    DatafusionContext::with_delta_catalog(|catalog| {
        if catalog.schema(&temp_schema_name).is_none() {
            let schema_provider = Arc::new(ObjectStoreSchemaProvider::new()?);
            catalog.register_schema(&temp_schema_name, schema_provider)?;
        }
        Ok(())
    })?;

    let _ = DatafusionContext::with_delta_schema_provider(temp_schema_name, |provider| {
        Ok(provider.register_table(table_name.clone(), listing_table))
    })?;

    spi::Spi::run(&format!("CREATE TEMP TABLE {} USING parquet", table_name))?;
    spi::Spi::run(&format!("DROP TABLE {}", DUMMY_TABLE_NAME))?;
    Ok(())
}
