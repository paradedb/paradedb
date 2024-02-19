use deltalake::datafusion::catalog::CatalogProvider;
use pgrx::*;

use crate::datafusion::session::ParadeSessionContext;
use crate::errors::{NotFound, ParadeError};

#[pg_extern]
pub fn object_store_schema(schema_name: &str) -> iter::TableIterator<(name!(table, String),)> {
    let table_names = object_store_schema_impl(schema_name).unwrap_or_else(|err| {
        panic!("{}", err);
    });

    iter::TableIterator::new(table_names.into_iter().map(|table| (table,)))
}

#[inline]
fn object_store_schema_impl(schema_name: &str) -> Result<Vec<String>, ParadeError> {
    ParadeSessionContext::with_object_store_catalog(|catalog| {
        let schema_provider = catalog
            .schema(schema_name)
            .ok_or(NotFound::Schema(schema_name.to_string()))?;

        Ok(schema_provider.table_names())
    })
}
