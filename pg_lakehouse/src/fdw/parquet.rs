use anyhow::Result;
use async_std::stream::StreamExt;
use async_std::task;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::physical_plan::SendableRecordBatchStream;
use datafusion::prelude::DataFrame;
use duckdb::params;
use pgrx::*;
use std::collections::HashMap;
use supabase_wrappers::prelude::*;
use url::Url;

use crate::datafusion::format::TableFormat;
use crate::duckdb::connection::duckdb_connection;
use crate::fdw::options::*;

use super::base::*;

#[wrappers_fdw(
    author = "ParadeDB",
    website = "https://github.com/paradedb/paradedb",
    error_type = "BaseFdwError"
)]
pub(crate) struct LocalFileFdw {
    dataframe: Option<DataFrame>,
    stream: Option<SendableRecordBatchStream>,
    current_batch: Option<RecordBatch>,
    current_batch_index: usize,
    target_columns: Vec<Column>,
}

enum ParquetOption {
    BinaryAsString,
    EncryptionConfig,
    FileName,
    FileRowNumber,
    Files,
    HivePartitioning,
    UnionByName,
}

impl ParquetOption {
    pub fn as_str(&self) -> &str {
        match self {
            Self::BinaryAsString => "binary_as_string",
            Self::EncryptionConfig => "encryption_config",
            Self::FileName => "file_name",
            Self::FileRowNumber => "file_row_number",
            Self::Files => "files",
            Self::HivePartitioning => "hive_partitioning",
            Self::UnionByName => "union_by_name",
        }
    }

    pub fn is_required(&self) -> bool {
        match self {
            Self::BinaryAsString => false,
            Self::EncryptionConfig => false,
            Self::FileName => false,
            Self::FileRowNumber => false,
            Self::Files => true,
            Self::HivePartitioning => false,
            Self::UnionByName => false,
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Self::BinaryAsString,
            Self::EncryptionConfig,
            Self::FileName,
            Self::FileRowNumber,
            Self::Files,
            Self::HivePartitioning,
            Self::UnionByName,
        ]
        .into_iter()
    }
}

impl BaseFdw for LocalFileFdw {
    fn register_object_store(
        url: &Url,
        format: TableFormat,
        _server_options: HashMap<String, String>,
        _user_mapping_options: HashMap<String, String>,
    ) -> Result<()> {
        let conn = duckdb_connection();
        conn.execute(
            format!(
                "CREATE VIEW IF NOT EXISTS hits AS SELECT * FROM read_parquet('{}')",
                url.path().to_string()
            )
            .as_str(),
            [],
        )?;

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

impl ForeignDataWrapper<BaseFdwError> for LocalFileFdw {
    fn new(
        table_options: HashMap<String, String>,
        server_options: HashMap<String, String>,
        user_mapping_options: HashMap<String, String>,
    ) -> Result<Self, BaseFdwError> {
        let path = require_option(TableOption::Path.as_str(), &table_options)?;
        let format = require_option_or(TableOption::Format.as_str(), &table_options, "");

        LocalFileFdw::register_object_store(
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
                FOREIGN_SERVER_RELATION_ID => {}
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
                unsupported => {
                    return Err(BaseFdwError::UnsupportedFdwOid(PgOid::from(unsupported)))
                }
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
        Ok(())
        // task::block_on(self.begin_scan_impl(_quals, columns, _sorts, limit, options))
    }

    fn iter_scan(&mut self, row: &mut Row) -> Result<Option<()>, BaseFdwError> {
        task::block_on(self.iter_scan_impl(row))
    }

    fn end_scan(&mut self) -> Result<(), BaseFdwError> {
        self.end_scan_impl()
    }
}
