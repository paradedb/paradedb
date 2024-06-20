use anyhow::Result;
use async_std::task;
use duckdb::arrow::array::RecordBatch;
use pgrx::*;
use std::collections::HashMap;
use supabase_wrappers::prelude::*;

use super::base::*;
use crate::duckdb::parquet::ParquetOption;

#[wrappers_fdw(
    author = "ParadeDB",
    website = "https://github.com/paradedb/paradedb",
    error_type = "BaseFdwError"
)]
pub(crate) struct ParquetFdw {
    current_batch: Option<RecordBatch>,
    current_batch_index: usize,
    scan_started: bool,
    sql: Option<String>,
    target_columns: Vec<Column>,
    user_mapping_options: HashMap<String, String>,
}

impl BaseFdw for ParquetFdw {
    fn get_current_batch(&self) -> Option<RecordBatch> {
        self.current_batch.clone()
    }

    fn get_current_batch_index(&self) -> usize {
        self.current_batch_index
    }

    fn get_scan_started(&self) -> bool {
        self.scan_started
    }

    fn get_sql(&self) -> Option<String> {
        self.sql.clone()
    }

    fn get_target_columns(&self) -> Vec<Column> {
        self.target_columns.clone()
    }

    fn get_user_mapping_options(&self) -> HashMap<String, String> {
        self.user_mapping_options.clone()
    }

    fn set_current_batch(&mut self, batch: Option<RecordBatch>) {
        self.current_batch = batch;
    }

    fn set_current_batch_index(&mut self, index: usize) {
        self.current_batch_index = index;
    }

    fn set_scan_started(&mut self) {
        self.scan_started = true;
    }

    fn set_sql(&mut self, sql: Option<String>) {
        self.sql = sql;
    }

    fn set_target_columns(&mut self, columns: &[Column]) {
        self.target_columns = columns.to_vec();
    }
}

impl ForeignDataWrapper<BaseFdwError> for ParquetFdw {
    fn new(
        _table_options: HashMap<String, String>,
        _server_options: HashMap<String, String>,
        user_mapping_options: HashMap<String, String>,
    ) -> Result<Self, BaseFdwError> {
        Ok(Self {
            current_batch: None,
            current_batch_index: 0,
            scan_started: false,
            sql: None,
            target_columns: Vec::new(),
            user_mapping_options,
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
                // TODO: Sanitize user mapping options
                _ => {}
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
        Ok(task::block_on(
            self.begin_scan_impl(quals, columns, sorts, limit, options),
        )?)
    }

    fn iter_scan(&mut self, row: &mut Row) -> Result<Option<()>, BaseFdwError> {
        Ok(task::block_on(self.iter_scan_impl(row))?)
    }

    fn end_scan(&mut self) -> Result<(), BaseFdwError> {
        self.end_scan_impl();
        Ok(())
    }

    fn explain(&self) -> Result<Option<Vec<(String, String)>>, BaseFdwError> {
        Ok(self.explain_impl()?)
    }
}
