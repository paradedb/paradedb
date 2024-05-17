use async_std::stream::StreamExt;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::physical_plan::SendableRecordBatchStream;
use object_store_opendal::OpendalStore;
use opendal::services::Gcs;
use opendal::Operator;
use pgrx::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use url::Url;

use crate::datafusion::context::ContextError;
use crate::datafusion::session::Session;
use crate::fdw::options::*;

use super::base::*;

#[wrappers_fdw(
    author = "ParadeDB",
    website = "https://github.com/paradedb/paradedb",
    error_type = "BaseFdwError"
)]
pub(crate) struct GcsFdw {
    stream: Option<SendableRecordBatchStream>,
    current_batch: Option<RecordBatch>,
    current_batch_index: usize,
    target_columns: Vec<Column>,
}

enum GcsServerOption {
    Bucket,
    DefaultStorageClass,
    Endpoint,
    PredefinedAcl,
    Root,
}

impl GcsServerOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Bucket => "bucket",
            Self::DefaultStorageClass => "default_storage_class",
            Self::Endpoint => "endpoint",
            Self::PredefinedAcl => "predefined_acl",
            Self::Root => "root",
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Self::Bucket => true,
            Self::DefaultStorageClass => false,
            Self::Endpoint => false,
            Self::PredefinedAcl => false,
            Self::Root => false,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Self::Bucket,
            Self::DefaultStorageClass,
            Self::Endpoint,
            Self::PredefinedAcl,
            Self::Root,
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
        let user_mapping_options = options.user_mapping_options();

        let mut builder = Gcs::default();
        let bucket = require_option(GcsServerOption::Bucket.as_str(), server_options)?;
        builder.bucket(bucket);

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

        if let Some(root) = server_options.get(GcsServerOption::Root.as_str()) {
            builder.root(root);
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
        server_options: HashMap<String, String>,
        user_mapping_options: HashMap<String, String>,
    ) -> Result<(), ContextError> {
        let context = Session::session_context()?;

        let builder = Gcs::try_from(ServerOptions::new(
            server_options.clone(),
            user_mapping_options.clone(),
        ))?;

        let operator = Operator::new(builder)?.finish();
        let object_store = Arc::new(OpendalStore::new(operator));
        let bucket = require_option(GcsServerOption::Bucket.as_str(), &server_options)?;

        let mut path = match server_options.get(GcsServerOption::Root.as_str()) {
            Some(root) => {
                let mut path = PathBuf::from(bucket);
                path.push(root);
                path
            }
            None => PathBuf::from(bucket),
        };

        if let Some(path_str) = path.to_str() {
            if let Some(stripped) = path_str.strip_prefix('/') {
                path = PathBuf::from(stripped);
            }
        }

        let url = format!("gs://{}", path.to_string_lossy());

        context
            .runtime_env()
            .register_object_store(&Url::parse(&url)?, object_store);

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

    fn set_stream(&mut self, stream: Option<SendableRecordBatchStream>) {
        self.stream = stream;
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
        server_options: HashMap<String, String>,
        user_mapping_options: HashMap<String, String>,
    ) -> Result<Self, BaseFdwError> {
        GcsFdw::register_object_store(server_options, user_mapping_options)?;

        Ok(Self {
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
