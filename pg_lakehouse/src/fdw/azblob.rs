use anyhow::{Error, Result};
use async_std::stream::StreamExt;
use async_std::task;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::physical_plan::SendableRecordBatchStream;
use datafusion::prelude::DataFrame;
use object_store_opendal::OpendalStore;
use opendal::services::Azblob;
use opendal::Operator;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use url::Url;

use crate::datafusion::format::TableFormat;
use crate::datafusion::session::Session;
use crate::fdw::options::*;

use super::base::*;

#[wrappers_fdw(
    author = "ParadeDB",
    website = "https://github.com/paradedb/paradedb",
    error_type = "BaseFdwError"
)]
pub(crate) struct AzblobFdw {
    dataframe: Option<DataFrame>,
    stream: Option<SendableRecordBatchStream>,
    current_batch: Option<RecordBatch>,
    current_batch_index: usize,
    target_columns: Vec<Column>,
}

enum AzblobServerOption {
    EncryptionAlgorithm,
    Endpoint,
}

impl AzblobServerOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::EncryptionAlgorithm => "encryption_algorithm",
            Self::Endpoint => "endpoint",
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Self::EncryptionAlgorithm => false,
            Self::Endpoint => true,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [Self::EncryptionAlgorithm, Self::Endpoint].into_iter()
    }
}

enum AzblobUserMappingOption {
    AccountKey,
    AccountName,
    ConnectionString,
    CustomerProvidedKey,
    EncryptionKey,
    EncyptionKeySha256,
    SasToken,
}

impl AzblobUserMappingOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::AccountKey => "account_key",
            Self::AccountName => "account_name",
            Self::ConnectionString => "connection_string",
            Self::CustomerProvidedKey => "customer_provided_key",
            Self::EncryptionKey => "encryption_key",
            Self::EncyptionKeySha256 => "encryption_key_sha256",
            Self::SasToken => "sas_token",
        }
    }
}

impl TryFrom<ObjectStoreConfig> for Azblob {
    type Error = Error;

    fn try_from(options: ObjectStoreConfig) -> Result<Self> {
        let server_options = options.server_options();
        let url = options.url();
        let format = options.format().clone();
        let user_mapping_options = options.user_mapping_options();

        let mut builder = Azblob::default();

        if format == TableFormat::Delta {
            builder.root(url.path());
        }

        if let Some(connection_string) =
            user_mapping_options.get(AzblobUserMappingOption::ConnectionString.as_str())
        {
            builder = Azblob::from_connection_string(connection_string)?;
        }

        if let Some(container) = url.host_str() {
            builder.container(container);
        }

        if let Some(account_key) =
            user_mapping_options.get(AzblobUserMappingOption::AccountKey.as_str())
        {
            builder.account_key(account_key);
        }

        if let Some(account_name) =
            user_mapping_options.get(AzblobUserMappingOption::AccountName.as_str())
        {
            builder.account_name(account_name);
        }

        if let Some(customer_provided_key) =
            user_mapping_options.get(AzblobUserMappingOption::CustomerProvidedKey.as_str())
        {
            builder.server_side_encryption_with_customer_key(customer_provided_key.as_bytes());
        }

        if let Some(encryption_key) =
            user_mapping_options.get(AzblobUserMappingOption::EncryptionKey.as_str())
        {
            builder.encryption_key(encryption_key);
        }

        if let Some(encryption_key_sha256) =
            user_mapping_options.get(AzblobUserMappingOption::EncyptionKeySha256.as_str())
        {
            builder.encryption_key_sha256(encryption_key_sha256);
        }

        if let Some(sas_token) =
            user_mapping_options.get(AzblobUserMappingOption::SasToken.as_str())
        {
            builder.sas_token(sas_token);
        }

        if let Some(encryption_algorithm) =
            server_options.get(AzblobServerOption::EncryptionAlgorithm.as_str())
        {
            builder.encryption_algorithm(encryption_algorithm);
        }

        if let Some(endpoint) = server_options.get(AzblobServerOption::Endpoint.as_str()) {
            builder.endpoint(endpoint);
        }

        Ok(builder)
    }
}

impl BaseFdw for AzblobFdw {
    fn register_object_store(
        url: &Url,
        format: TableFormat,
        server_options: HashMap<String, String>,
        user_mapping_options: HashMap<String, String>,
    ) -> Result<()> {
        let context = Session::session_context()?;

        let builder = Azblob::try_from(ObjectStoreConfig::new(
            url,
            format,
            server_options.clone(),
            user_mapping_options.clone(),
        ))?;

        let operator = Operator::new(builder)?.finish();
        let object_store = Arc::new(OpendalStore::new(operator));

        context
            .runtime_env()
            .register_object_store(url, object_store);

        Ok(())
    }

    fn get_current_batch(&self) -> Option<RecordBatch> {
        self.current_batch.clone()
    }

    fn get_current_batch_index(&self) -> usize {
        self.current_batch_index
    }

    fn get_target_columns(&self) -> Vec<Column> {
        self.target_columns.clone()
    }

    fn set_current_batch(&mut self, batch: Option<RecordBatch>) {
        self.current_batch = batch;
    }

    fn set_current_batch_index(&mut self, index: usize) {
        self.current_batch_index = index;
    }

    fn set_dataframe(&mut self, dataframe: DataFrame) {
        self.dataframe = Some(dataframe);
    }

    async fn create_stream(&mut self) -> Result<()> {
        if self.stream.is_none() {
            self.stream = Some(
                self.dataframe
                    .clone()
                    .ok_or(BaseFdwError::DataFrameNotFound)?
                    .execute_stream()
                    .await?,
            );
        }

        Ok(())
    }

    fn clear_stream(&mut self) {
        self.stream = None;
    }

    fn set_target_columns(&mut self, columns: &[Column]) {
        self.target_columns = columns.to_vec();
    }

    async fn get_next_batch(&mut self) -> Result<Option<RecordBatch>> {
        match self
            .stream
            .as_mut()
            .ok_or(BaseFdwError::StreamNotFound)?
            .next()
            .await
        {
            Some(Ok(batch)) => Ok(Some(batch)),
            None => Ok(None),
            Some(Err(err)) => Err(err.into()),
        }
    }
}

impl ForeignDataWrapper<BaseFdwError> for AzblobFdw {
    fn new(
        table_options: HashMap<String, String>,
        server_options: HashMap<String, String>,
        user_mapping_options: HashMap<String, String>,
    ) -> Result<Self, BaseFdwError> {
        let path = require_option(TableOption::Path.as_str(), &table_options)?;
        let format = require_option_or(TableOption::Format.as_str(), &table_options, "");

        AzblobFdw::register_object_store(
            &Url::parse(path)?,
            TableFormat::from(format),
            server_options,
            user_mapping_options,
        )?;

        Ok(Self {
            dataframe: None,
            current_batch: None,
            current_batch_index: 0,
            stream: None,
            target_columns: Vec::new(),
        })
    }

    fn validator(
        opt_list: Vec<Option<String>>,
        catalog: Option<pg_sys::Oid>,
    ) -> Result<(), BaseFdwError> {
        if let Some(oid) = catalog {
            match oid {
                FOREIGN_DATA_WRAPPER_RELATION_ID => {}
                FOREIGN_SERVER_RELATION_ID => {
                    let valid_options: Vec<String> = AzblobServerOption::iter()
                        .map(|opt| opt.as_str().to_string())
                        .collect();

                    validate_options(opt_list.clone(), valid_options)?;

                    for opt in AzblobServerOption::iter() {
                        if opt.is_required() {
                            check_options_contain(&opt_list, opt.as_str())?;
                        }
                    }
                }
                FOREIGN_TABLE_RELATION_ID => {
                    let valid_options: Vec<String> = TableOption::iter()
                        .map(|opt| opt.as_str().to_string())
                        .collect();

                    validate_options(opt_list.clone(), valid_options)?;

                    for opt in TableOption::iter() {
                        if opt.is_required() {
                            check_options_contain(&opt_list, opt.as_str())?;
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn begin_scan(
        &mut self,
        _quals: &[Qual],
        columns: &[Column],
        _sorts: &[Sort],
        limit: &Option<Limit>,
        options: HashMap<String, String>,
    ) -> Result<(), BaseFdwError> {
        task::block_on(self.begin_scan_impl(_quals, columns, _sorts, limit, options))
    }

    fn iter_scan(&mut self, row: &mut Row) -> Result<Option<()>, BaseFdwError> {
        task::block_on(self.iter_scan_impl(row))
    }

    fn end_scan(&mut self) -> Result<(), BaseFdwError> {
        self.end_scan_impl()
    }
}
