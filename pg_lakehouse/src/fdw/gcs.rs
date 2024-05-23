use async_std::stream::StreamExt;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::DataFusionError;
use datafusion::physical_plan::SendableRecordBatchStream;
use datafusion::prelude::DataFrame;
use object_store_opendal::OpendalStore;
use opendal::services::Gcs;
use opendal::Operator;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use url::Url;

use crate::datafusion::context::ContextError;
use crate::datafusion::format::TableFormat;
use crate::datafusion::session::Session;
use crate::fdw::options::*;

use super::base::*;

#[wrappers_fdw(
    author = "ParadeDB",
    website = "https://github.com/paradedb/paradedb",
    error_type = "BaseFdwError"
)]
pub(crate) struct GcsFdw {
    dataframe: Option<DataFrame>,
    stream: Option<SendableRecordBatchStream>,
    current_batch: Option<RecordBatch>,
    current_batch_index: usize,
    target_columns: Vec<Column>,
}

enum GcsServerOption {
    DefaultStorageClass,
    Endpoint,
    PredefinedAcl,
}

impl GcsServerOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::DefaultStorageClass => "default_storage_class",
            Self::Endpoint => "endpoint",
            Self::PredefinedAcl => "predefined_acl",
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Self::DefaultStorageClass => false,
            Self::Endpoint => false,
            Self::PredefinedAcl => false,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Self::DefaultStorageClass,
            Self::Endpoint,
            Self::PredefinedAcl,
        ]
        .into_iter()
    }
}

enum GcsUserMappingOption {
    Credential,
    CredentialPath,
    Scope,
    ServiceAccount,
}

impl GcsUserMappingOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Credential => "credential",
            Self::CredentialPath => "credential_path",
            Self::Scope => "scope",
            Self::ServiceAccount => "service_account",
        }
    }
}

impl TryFrom<ServerOptions> for Gcs {
    type Error = ContextError;

    fn try_from(options: ServerOptions) -> Result<Self, Self::Error> {
        let server_options = options.server_options();
        let url = options.url();
        let user_mapping_options = options.user_mapping_options();

        let mut builder = Gcs::default();

        if let Some(bucket) = url.host_str() {
            builder.bucket(bucket);
        }

        if let Some(credential) =
            user_mapping_options.get(GcsUserMappingOption::Credential.as_str())
        {
            builder.credential(credential);
        }

        if let Some(credential_path) =
            user_mapping_options.get(GcsUserMappingOption::CredentialPath.as_str())
        {
            builder.credential_path(credential_path);
        }

        if let Some(default_storage_class) =
            server_options.get(GcsServerOption::DefaultStorageClass.as_str())
        {
            builder.default_storage_class(default_storage_class);
        }

        if let Some(endpoint) = server_options.get(GcsServerOption::Endpoint.as_str()) {
            builder.endpoint(endpoint);
        }

        if let Some(predefined_acl) = server_options.get(GcsServerOption::PredefinedAcl.as_str()) {
            builder.predefined_acl(predefined_acl);
        }

        if let Some(scope) = user_mapping_options.get(GcsUserMappingOption::Scope.as_str()) {
            builder.scope(scope);
        }

        if let Some(service_account) =
            user_mapping_options.get(GcsUserMappingOption::ServiceAccount.as_str())
        {
            builder.service_account(service_account);
        }

        Ok(builder)
    }
}

impl BaseFdw for GcsFdw {
    fn register_object_store(
        url: &Url,
        _format: TableFormat,
        server_options: HashMap<String, String>,
        user_mapping_options: HashMap<String, String>,
    ) -> Result<(), ContextError> {
        let context = Session::session_context()?;

        let builder = Gcs::try_from(ServerOptions::new(
            url,
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

    async fn set_stream(&mut self) -> Result<(), DataFusionError> {
        if self.stream.is_none() {
            self.stream = Some(self.dataframe.clone().unwrap().execute_stream().await?);
        }

        Ok(())
    }

    fn set_target_columns(&mut self, columns: &[Column]) {
        self.target_columns = columns.to_vec();
    }

    async fn get_next_batch(&mut self) -> Result<Option<RecordBatch>, BaseFdwError> {
        match self
            .stream
            .as_mut()
            .ok_or(BaseFdwError::StreamNotFound)?
            .next()
            .await
        {
            Some(Ok(batch)) => Ok(Some(batch)),
            None => Ok(None),
            Some(Err(err)) => Err(BaseFdwError::DataFusionError(err)),
        }
    }
}

impl ForeignDataWrapper<BaseFdwError> for GcsFdw {
    fn new(
        table_options: HashMap<String, String>,
        server_options: HashMap<String, String>,
        user_mapping_options: HashMap<String, String>,
    ) -> Result<Self, BaseFdwError> {
        let path = require_option(TableOption::Path.as_str(), &table_options)?;
        let format = require_option_or(TableOption::Format.as_str(), &table_options, "");

        GcsFdw::register_object_store(
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
                    let valid_options: Vec<String> = GcsServerOption::iter()
                        .map(|opt| opt.as_str().to_string())
                        .collect();

                    validate_options(opt_list.clone(), valid_options)?;

                    for opt in GcsServerOption::iter() {
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
        self.begin_scan_impl(_quals, columns, _sorts, limit, options)
    }

    fn iter_scan(&mut self, row: &mut Row) -> Result<Option<()>, BaseFdwError> {
        self.iter_scan_impl(row)
    }

    fn end_scan(&mut self) -> Result<(), BaseFdwError> {
        self.end_scan_impl()
    }
}
