use async_std::task;
use datafusion::common::arrow::datatypes::{DataType, Field, SchemaBuilder};
use datafusion::common::DataFusionError;
use datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use datafusion::datasource::TableProvider;
use datafusion::execution::context::SessionState;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use thiserror::Error;

use super::format::*;
use super::options::*;

pub fn create_table_provider(
    table_options: HashMap<String, String>,
    mut oid_map: HashMap<usize, pg_sys::Oid>,
    state: &SessionState,
) -> Result<Arc<dyn TableProvider>, TableError> {
    let path = require_option(TableOption::Path.as_str(), &table_options)?;
    let extension = require_option(TableOption::Extension.as_str(), &table_options)?;

    let listing_url = ListingTableUrl::parse(path)?;
    let listing_options = ListingOptions::try_from(FileFormat(extension.to_string()))?;

    let inferred_schema = task::block_on(listing_options.infer_schema(state, &listing_url))?;
    let mut schema_builder = SchemaBuilder::new();

    for (index, field) in inferred_schema.fields().iter().enumerate() {
        match oid_map.remove(&index) {
            Some(oid) => {
                // Some types like DATE and TIMESTAMP get incorrectly inferred as
                // Int32/Int64, so we need to override them
                let data_type = match (oid, field.data_type()) {
                    (pg_sys::DATEOID, _) => DataType::Int32,
                    (pg_sys::TIMESTAMPOID, _) => DataType::Int64,
                    (_, data_type) => data_type.clone(),
                };
                schema_builder.push(Field::new(field.name(), data_type, field.is_nullable()))
            }
            None => schema_builder.push(field.clone()),
        };
    }

    let updated_schema = Arc::new(schema_builder.finish());

    let listing_config = ListingTableConfig::new(listing_url)
        .with_listing_options(listing_options)
        .with_schema(updated_schema);

    let listing_table = ListingTable::try_new(listing_config)?;

    Ok(Arc::new(listing_table) as Arc<dyn TableProvider>)
}

#[derive(Error, Debug)]
pub enum TableError {
    #[error(transparent)]
    DataFusion(#[from] DataFusionError),

    #[error(transparent)]
    FileFormat(#[from] FileFormatError),

    #[error(transparent)]
    Option(#[from] supabase_wrappers::options::OptionsError),
}
