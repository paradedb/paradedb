mod format;
mod options;

use async_std::task;
use datafusion::common::DataFusionError;
use datafusion::datasource::listing::{ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl};
use datafusion::physical_plan::SendableRecordBatchStream;
use datafusion::prelude::SessionContext;
use object_store::aws::AmazonS3Builder;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use thiserror::Error;
use url::Url;

use crate::s3::format::*;
use crate::s3::options::*;

#[wrappers_fdw(
    author = "ParadeDB",
    website = "https://github.com/paradedb/paradedb",
    error_type = "S3FdwError"
)]

pub(crate) struct S3Fdw {
    stream: Option<SendableRecordBatchStream>,
    context: SessionContext,
}

impl From<S3FdwError> for pg_sys::panic::ErrorReport {
    fn from(value: S3FdwError) -> Self {
        pg_sys::panic::ErrorReport::new(PgSqlErrorCode::ERRCODE_FDW_ERROR, format!("{}", value), "")
    }
}

impl ForeignDataWrapper<S3FdwError> for S3Fdw {
    fn new(options: &HashMap<String, String>) -> Result<Self, S3FdwError> {
        // Create S3 ObjectStore
        let url = require_option(ServerOption::Url.as_str(), options)?;
        let region = require_option(ServerOption::Region.as_str(), options)?;

        let mut builder = AmazonS3Builder::new().with_url(url).with_region(region);

        if let Some(access_key_id) = options.get(ServerOption::AccessKeyId.as_str()) {
            builder = builder.clone().with_access_key_id(access_key_id.as_str());
        }

        if let Some(secret_access_key) = options.get(ServerOption::SecretAccessKey.as_str()) {
            builder = builder.with_secret_access_key(secret_access_key.as_str());
        }

        if let Some(session_token) = options.get(ServerOption::SessionToken.as_str()) {
            builder = builder.with_token(session_token.as_str());
        }

        if let Some(endpoint) = options.get(ServerOption::Endpoint.as_str()) {
            builder = builder.with_endpoint(endpoint.as_str());
        }

        if let Some(allow_http) = options.get(ServerOption::AllowHttp.as_str()) {
            if allow_http == "true" {
                builder = builder.with_allow_http(true);
            }
        }

        // Create SessionContext with ObjectStore
        let context = SessionContext::new();
        context
            .runtime_env()
            .register_object_store(&Url::parse(&url)?, Arc::new(builder.build()?));

        Ok(Self {
            stream: None,
            context,
        })
    }

    fn validator(
        opt_list: Vec<Option<String>>,
        catalog: Option<pg_sys::Oid>,
    ) -> Result<(), S3FdwError> {
        if let Some(oid) = catalog {
            match oid {
                FOREIGN_DATA_WRAPPER_RELATION_ID => {}
                FOREIGN_SERVER_RELATION_ID => {
                    for opt in ServerOption::iter() {
                        if opt.is_required() {
                            check_options_contain(&opt_list, opt.as_str())?;
                        }
                    }
                }
                FOREIGN_TABLE_RELATION_ID => {
                    for opt in TableOption::iter() {
                        if opt.is_required() {
                            check_options_contain(&opt_list, opt.as_str())?;
                        }
                    }
                }
                unsupported => return Err(S3FdwError::UnsupportedFdwOid(unsupported)),
            }
        }

        Ok(())
    }

    fn begin_scan(
        &mut self,
        _quals: &[Qual],
        _columns: &[Column],
        _sorts: &[Sort],
        _limit: &Option<Limit>,
        options: &HashMap<String, String>,
    ) -> Result<(), S3FdwError> {
        // Register ListingTable with SessionContext
        let table_name = require_option(TableOption::Table.as_str(), options)?;
        let table_url = require_option(TableOption::Url.as_str(), options)?;
        let format = require_option(TableOption::Format.as_str(), options)?;
        let listing_url = ListingTableUrl::parse(table_url)?;
        let listing_options = ListingOptions::try_from(FileFormat(format.to_string()))?;
        let listing_config = task::block_on(
            ListingTableConfig::new(listing_url)
                .with_listing_options(listing_options)
                .infer_schema(&self.context.state()),
        )?;

        let listing_table = ListingTable::try_new(listing_config)?;
        self.context.register_table(table_name, Arc::new(listing_table))?;

        Ok(())
    }

    fn iter_scan(&mut self, _row: &mut Row) -> Result<Option<()>, S3FdwError> {
        Ok(None)
    }

    fn end_scan(&mut self) -> Result<(), S3FdwError> {
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum S3FdwError {
    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error(transparent)]
    FileFormatError(#[from] FileFormatError),

    #[error(transparent)]
    ObjectStoreError(#[from] object_store::Error),

    #[error(transparent)]
    OptionsError(#[from] OptionsError),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),

    #[error("Received unsupported FDW oid {0:?}")]
    UnsupportedFdwOid(pg_sys::Oid),
}
