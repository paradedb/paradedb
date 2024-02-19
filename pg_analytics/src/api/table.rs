use async_std::task;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::catalog::CatalogProvider;
use deltalake::datafusion::common::arrow::datatypes::Schema;
use pgrx::*;
use std::ffi::CStr;
use std::sync::Arc;

use crate::datafusion::datatype::DatafusionTypeTranslator;
use crate::datafusion::schema::TempSchemaProvider;
use crate::datafusion::session::ParadeSessionContext;
use crate::errors::{NotFound, ParadeError};

const DUMMY_TABLE_NAME: &str = "paradedb_dummy_temp_table";

#[pg_extern]
pub fn list_tables(schema_name: &str) -> iter::TableIterator<(name!(table, String),)> {
    let table_names = list_tables_impl(schema_name).unwrap_or_else(|err| {
        panic!("{}", err);
    });

    iter::TableIterator::new(table_names.into_iter().map(|table| (table,)))
}

#[inline]
fn list_tables_impl(schema_name: &str) -> Result<Vec<String>, ParadeError> {
    ParadeSessionContext::with_object_store_catalog(|catalog| {
        let schema_provider = catalog
            .schema(schema_name)
            .ok_or(NotFound::Schema(schema_name.to_string()))?;

        Ok(schema_provider.table_names())
    })
}

extension_sql!(
    r#"
    CREATE OR REPLACE PROCEDURE register_temp_table(
        table_name TEXT,
        foreign_table_name TEXT,
        schema_name TEXT
    ) 
    LANGUAGE C AS 'MODULE_PATHNAME', 'register_temp_table';
    "#,
    name = "register_temp_table"
);
#[pg_guard]
#[no_mangle]
pub extern "C" fn register_temp_table(fcinfo: pg_sys::FunctionCallInfo) {
    register_temp_table_impl(fcinfo).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

fn register_temp_table_impl(fcinfo: pg_sys::FunctionCallInfo) -> Result<(), ParadeError> {
    let table_name: String = unsafe { fcinfo::pg_getarg(fcinfo, 0).unwrap() };
    let foreign_table_name: String = unsafe { fcinfo::pg_getarg(fcinfo, 1).unwrap() };
    let schema_name: String = unsafe { fcinfo::pg_getarg(fcinfo, 2).unwrap() };

    let temp_schema_oid = unsafe {
        match direct_function_call::<pg_sys::Oid>(pg_sys::pg_my_temp_schema, &[]) {
            Some(pg_sys::InvalidOid) => {
                spi::Spi::run(&format!(
                    "CREATE TEMP TABLE IF NOT EXISTS {} (a int)",
                    DUMMY_TABLE_NAME
                ))?;

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

    let listing_table = ParadeSessionContext::with_object_store_catalog(|catalog| {
        let schema_provider = catalog
            .schema(&schema_name)
            .ok_or(NotFound::Schema(schema_name.to_string()))?;

        task::block_on(schema_provider.table(&foreign_table_name))
            .ok_or(NotFound::Table(foreign_table_name).into())
    })?;

    ParadeSessionContext::with_postgres_catalog(|catalog| {
        if catalog.schema(temp_schema_name).is_none() {
            let schema_provider = Arc::new(TempSchemaProvider::new()?);
            catalog.register_schema(temp_schema_name, schema_provider)?;
        }
        Ok(())
    })?;

    let _ = ParadeSessionContext::with_temp_schema_provider(temp_schema_name, |provider| {
        Ok(provider.register_table(table_name.clone(), listing_table.clone()))
    })?;

    let statement = create_temp_table_statement(listing_table.schema(), &table_name)?;

    spi::Spi::run(&statement)?;
    spi::Spi::run(&format!("DROP TABLE IF EXISTS {}", DUMMY_TABLE_NAME))?;
    Ok(())
}

#[inline]
fn create_temp_table_statement(
    schema: Arc<Schema>,
    table_name: &str,
) -> Result<String, ParadeError> {
    let mut create_table = String::new();
    create_table.push_str("CREATE TEMP TABLE ");
    create_table.push_str(table_name);
    create_table.push_str(" (");

    let fields = schema.as_ref().fields();
    for (i, field) in fields.iter().enumerate() {
        create_table.push_str(field.name());
        create_table.push(' ');
        create_table.push_str(&field.data_type().to_postgres_string()?);

        if !field.is_nullable() {
            create_table.push_str(" NOT NULL");
        }

        if i < fields.len() - 1 {
            create_table.push_str(", ");
        }
    }

    create_table.push_str(") USING parquet;");

    Ok(create_table)
}
