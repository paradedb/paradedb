use datafusion::common::DataFusionError;
use datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use datafusion::datasource::TableProvider;
use datafusion::execution::context::SessionState;
use deltalake::DeltaTableError;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use crate::fdw::format::*;
use crate::fdw::options::*;
use crate::types::schema::*;

pub async fn create_listing_provider(
    table_options: HashMap<String, String>,
    mut attribute_map: HashMap<usize, PgAttribute>,
    state: &SessionState,
) -> Result<Arc<dyn TableProvider>, TableProviderError> {
    let path = require_option(TableOption::Path.as_str(), &table_options)?;
    let extension = require_option(TableOption::Extension.as_str(), &table_options)?;

    let listing_url = ListingTableUrl::parse(path)?;
    let listing_options = ListingOptions::try_from(FileExtension(extension.to_string()))?;
    let schema = listing_options.infer_schema(state, &listing_url).await?;

    for (index, field) in schema.fields().iter().enumerate() {
        if let Some(attribute) = attribute_map.remove(&index) {
            can_convert_to_attribute(field, attribute)?;
        }
    }

    let listing_config = ListingTableConfig::new(listing_url)
        .with_listing_options(listing_options)
        .with_schema(schema);

    let listing_table = ListingTable::try_new(listing_config)?;

    Ok(Arc::new(listing_table) as Arc<dyn TableProvider>)
}

pub async fn create_delta_provider(
    table_options: HashMap<String, String>,
    mut attribute_map: HashMap<usize, PgAttribute>,
) -> Result<Arc<dyn TableProvider>, TableProviderError> {
    let path = require_option(TableOption::Path.as_str(), &table_options)?;
    let provider = Arc::new(deltalake::open_table(path).await?) as Arc<dyn TableProvider>;
    let schema = (provider.clone()).schema();

    for (index, field) in schema.fields().iter().enumerate() {
        if let Some(attribute) = attribute_map.remove(&index) {
            can_convert_to_attribute(field, attribute)?;
        }
    }

    Ok(provider)
}

#[derive(Error, Debug)]
pub enum TableProviderError {
    #[error(transparent)]
    DataFusion(#[from] DataFusionError),

    #[error(transparent)]
    DeltaTable(#[from] DeltaTableError),

    #[error(transparent)]
    Format(#[from] FormatError),

    #[error(transparent)]
    Options(#[from] OptionsError),

    #[error(transparent)]
    Schema(#[from] SchemaError),
}
