use anyhow::{anyhow, Result};
use async_std::stream::StreamExt;
use async_std::sync::RwLock;
use async_std::task;
use datafusion::physical_plan::SendableRecordBatchStream;
use datafusion::prelude::DataFrame;
use duckdb::arrow::array::RecordBatch;
use duckdb::{params, Arrow, Connection, Statement};
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use url::Url;

use crate::datafusion::format::TableFormat;
use crate::duckdb::connection::{duckdb_connection, ConnectionWrapper};
use crate::fdw::options::*;

use super::base::*;

#[wrappers_fdw(
    author = "ParadeDB",
    website = "https://github.com/paradedb/paradedb",
    error_type = "BaseFdwError"
)]
pub(crate) struct ParquetFdw<'a> {
    connection: ConnectionWrapper<'a>,
    current_batch: Option<RecordBatch>,
    current_batch_index: usize,
    sql: Option<String>,
    target_columns: Vec<Column>,
}

pub enum ParquetOption {
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

impl BaseFdw for ParquetFdw<'_> {
    fn get_current_batch(&self) -> Option<RecordBatch> {
        self.current_batch.clone()
    }

    fn get_current_batch_index(&self) -> usize {
        self.current_batch_index
    }

    fn get_sql(&self) -> Option<String> {
        self.sql.clone()
    }

    fn get_target_columns(&self) -> Vec<Column> {
        self.target_columns.clone()
    }

    fn scan_started(&self) -> bool {
        self.connection.arrow.is_some()
    }

    fn set_arrow(&mut self) {
        // self.arrow = arrow;
    }

    fn set_current_batch(&mut self, batch: Option<RecordBatch>) {
        self.current_batch = batch;
    }

    fn set_current_batch_index(&mut self, index: usize) {
        self.current_batch_index = index;
    }

    fn set_sql(&mut self, sql: Option<String>) {
        self.sql = sql;
    }

    fn set_target_columns(&mut self, columns: &[Column]) {
        self.target_columns = columns.to_vec();
    }

    async fn get_next_batch(&mut self) -> Result<Option<RecordBatch>> {
        Ok(self
            .connection
            .arrow
            .as_mut()
            .ok_or_else(|| anyhow!("no Arrow batches found"))?
            .write()
            .await
            .next())
    }
}

impl ForeignDataWrapper<BaseFdwError> for ParquetFdw<'_> {
    fn new(
        table_options: HashMap<String, String>,
        server_options: HashMap<String, String>,
        user_mapping_options: HashMap<String, String>,
    ) -> Result<Self, BaseFdwError> {
        Ok(Self {
            connection: ConnectionWrapper::new()?,
            current_batch: None,
            current_batch_index: 0,
            sql: None,
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
                    let valid_options: Vec<String> = ParquetOption::iter()
                        .map(|opt| opt.as_str().to_string())
                        .collect();

                    validate_options(opt_list.clone(), valid_options)?;

                    for opt in ParquetOption::iter() {
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
        quals: &[Qual],
        columns: &[Column],
        sorts: &[Sort],
        limit: &Option<Limit>,
        options: HashMap<String, String>,
    ) -> Result<(), BaseFdwError> {
        task::block_on(self.begin_scan_impl(quals, columns, sorts, limit, options));
        Ok(())
    }

    fn iter_scan(&mut self, row: &mut Row) -> Result<Option<()>, BaseFdwError> {
        task::block_on(self.iter_scan_impl(row))
    }

    fn end_scan(&mut self) -> Result<(), BaseFdwError> {
        self.end_scan_impl()
    }
}
