use async_std::task;
use core::ffi::c_char;
use pgrx::*;

use deltalake::datafusion::catalog::CatalogProvider;
use deltalake::datafusion::sql::TableReference;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::schema::PermanentSchemaProvider;
use crate::errors::ParadeError;

#[pg_guard]
#[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
pub extern "C" fn deltalake_relation_set_new_filenode(
    rel: pg_sys::Relation,
    _newrnode: *const pg_sys::RelFileNode,
    persistence: c_char,
    _freezeXid: *mut pg_sys::TransactionId,
    _minmulti: *mut pg_sys::MultiXactId,
) {
    create_file_node(rel, persistence).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

#[pg_guard]
#[cfg(feature = "pg16")]
pub extern "C" fn deltalake_relation_set_new_filelocator(
    rel: pg_sys::Relation,
    _newrlocator: *const pg_sys::RelFileLocator,
    persistence: c_char,
    _freezeXid: *mut pg_sys::TransactionId,
    _minmulti: *mut pg_sys::MultiXactId,
) {
    create_file_node(rel, persistence).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

#[inline]
fn create_file_node(rel: pg_sys::Relation, persistence: c_char) -> Result<(), ParadeError> {
    let pg_relation = unsafe { PgRelation::from_pg(rel) };
    let table_name = pg_relation.name().to_string();
    let schema_name = pg_relation.namespace().to_string();

    match persistence as u8 {
        pg_sys::RELPERSISTENCE_TEMP => {
            // DatafusionContext::with_postgres_catalog(|catalog| {
            //     if catalog.schema(&schema_name).is_none() {
            //         let schema_provider = Arc::new(TempSchemaProvider::new());
            //         catalog.register_schema(&schema_name, schema_provider)?;
            //     }
            //     Ok(())
            // })?;

            // DatafusionContext::with_permanent_schema_provider(&schema_name, |provider| {
            //     task::block_on(provider.create_table(&pg_relation))
            // })
            Ok(())
        }
        _ => {
            let postgres_catalog_name = DatafusionContext::postgres_catalog_name()?;
            let schema_oid = pg_relation.namespace_oid();

            DatafusionContext::with_postgres_catalog(|catalog| {
                if catalog.schema(&schema_name).is_none() {
                    let schema_provider =
                        Arc::new(task::block_on(PermanentSchemaProvider::try_new(
                            &schema_name,
                            ParadeDirectory::schema_path(
                                DatafusionContext::postgres_catalog_oid()?,
                                schema_oid,
                            )?,
                        ))?);

                    catalog.register_schema(&schema_name, schema_provider)?;
                }

                Ok(())
            })?;

            let table_exists = DatafusionContext::with_session_context(|context| {
                let reference =
                    TableReference::full(postgres_catalog_name, schema_name.clone(), table_name);
                Ok(context.table_exist(reference)?)
            })?;

            // If the table already exists, then this function is being called as part of another
            // operation like VACUUM FULL or TRUNCATE and we don't want to create any new files
            if table_exists {
                return Ok(());
            }

            DatafusionContext::with_permanent_schema_provider(&schema_name, |provider| {
                task::block_on(provider.create_table(&pg_relation))
            })
        }
    }
}
