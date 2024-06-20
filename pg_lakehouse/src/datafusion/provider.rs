// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use datafusion::common::DataFusionError;
use datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use datafusion::datasource::TableProvider;
use deltalake::table::builder::ensure_table_uri;
use deltalake::table::builder::DeltaTableBuilder;
use deltalake::DeltaTableError;
use std::sync::Arc;
use thiserror::Error;
use url::Url;

use crate::schema::attribute::*;

use super::format::*;
use super::session::*;

pub struct ObjectStoreUrl(pub Url);

impl AsRef<Url> for ObjectStoreUrl {
    fn as_ref(&self) -> &Url {
        &self.0
    }
}

pub async fn create_listing_provider(
    path: &str,
    extension: &str,
) -> Result<Arc<dyn TableProvider>, TableProviderError> {
    let listing_url = ListingTableUrl::parse(path)?;
    let listing_options = ListingOptions::try_from(FileExtension(extension.to_string()))?;
    let context = Session::session_context()?;
    let schema = listing_options
        .infer_schema(&context.state(), &listing_url)
        .await?;
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

    deltalake::gcp::register_handlers(None);
    deltalake::aws::register_handlers(None);
    deltalake::azure::register_handlers(None);

    let url = Url::parse(path)?;
    let context = Session::session_context()?;
    let object_store = context.runtime_env().object_store(ObjectStoreUrl(url))?;
    let location = ensure_table_uri(path)?;

    let table = DeltaTableBuilder::from_valid_uri(path)?
        .with_storage_backend(object_store, location)
        .load()
        .await?;

    Ok(Arc::new(table) as Arc<dyn TableProvider>)
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

    #[error(transparent)]
    Session(#[from] SessionError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error(
        "File extension '{0}' is not supported for table format '{1}', extension must be 'parquet'"
    )]
    FileNotParquet(String, String),
}
