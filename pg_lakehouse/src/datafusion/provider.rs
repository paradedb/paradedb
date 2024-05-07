use datafusion::common::DataFusionError;
use datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use datafusion::datasource::TableProvider;
use datafusion::execution::context::SessionState;
use deltalake::DeltaTableError;
use std::sync::Arc;
use thiserror::Error;

use crate::datafusion::format::*;
use crate::schema::attribute::*;

pub async fn create_listing_provider(
    path: &str,
    extension: &str,
    state: &SessionState,
) -> Result<Arc<dyn TableProvider>, TableProviderError> {
    let listing_url = ListingTableUrl::parse(path)?;
    let listing_options = ListingOptions::try_from(FileExtension(extension.to_string()))?;
    let schema = listing_options.infer_schema(state, &listing_url).await?;
    let listing_config = ListingTableConfig::new(listing_url)
        .with_listing_options(listing_options)
        .with_schema(schema);
    let listing_table = ListingTable::try_new(listing_config)?;

    Ok(Arc::new(listing_table) as Arc<dyn TableProvider>)
}

pub async fn create_delta_provider(
    path: &str,
    extension: &str,
) -> Result<Arc<dyn TableProvider>, TableProviderError> {
    if extension != "parquet" {
        return Err(TableProviderError::FileNotParquet(
            extension.to_string(),
            "delta".to_string(),
        ));
    }

    Ok(Arc::new(deltalake::open_table(path).await?) as Arc<dyn TableProvider>)
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
    Options(#[from] supabase_wrappers::prelude::OptionsError),

    #[error(transparent)]
    Schema(#[from] SchemaError),

    #[error(
        "File extension '{0}' is not supported for table format '{1}', extension must be 'parquet'"
    )]
    FileNotParquet(String, String),
}
