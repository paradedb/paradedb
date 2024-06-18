use anyhow::{anyhow, Result};
use async_std::sync::RwLock;
use datafusion::arrow::error::ArrowError;
use datafusion::common::DataFusionError;
use deltalake::DeltaTableError;
use duckdb::arrow::array::RecordBatch;
use duckdb::Arrow;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use supabase_wrappers::prelude::*;
use thiserror::Error;
use url::Url;

use crate::datafusion::context::ContextError;
use crate::datafusion::format::*;
use crate::datafusion::provider::*;
use crate::datafusion::session::*;
use crate::duckdb::connection;
use crate::schema::attribute::*;
use crate::schema::cell::*;

pub trait BaseFdw {
    // Getter methods
    fn get_current_batch(&self) -> Option<RecordBatch>;
    fn get_current_batch_index(&self) -> usize;
    fn get_sql(&self) -> Option<String>;
    fn get_target_columns(&self) -> Vec<Column>;

    // Setter methods
    fn set_current_batch(&mut self, batch: Option<RecordBatch>);
    fn set_current_batch_index(&mut self, idx: usize);
    fn set_sql(&mut self, statement: Option<String>);
    fn set_target_columns(&mut self, columns: &[Column]);

    async fn begin_scan_impl(
        &mut self,
        _quals: &[Qual],
        columns: &[Column],
        sorts: &[Sort],
        limit: &Option<Limit>,
        options: HashMap<String, String>,
    ) -> Result<()> {
        self.set_target_columns(columns);

        let oid_u32: u32 = options
            .get(OPTS_TABLE_KEY)
            .ok_or_else(|| anyhow!("table oid not found"))?
            .parse()?;
        let table_oid = pg_sys::Oid::from(oid_u32);
        let pg_relation = unsafe { PgRelation::open(table_oid) };
        let schema_name = pg_relation.namespace();
        let table_name = pg_relation.name();

        let targets = if columns.is_empty() {
            "*".to_string()
        } else {
            columns
                .iter()
                .map(|c| c.name.clone())
                .collect::<Vec<String>>()
                .join(", ")
        };

        let mut sql = format!("SELECT {targets} FROM {schema_name}.{table_name}");

        if !sorts.is_empty() {
            let order_by = sorts
                .iter()
                .map(|sort| sort.deparse())
                .collect::<Vec<String>>()
                .join(", ");
            sql.push_str(&format!(" ORDER BY {}", order_by));
        }

        if let Some(limit) = limit {
            let real_limit = limit.offset + limit.count;
            sql.push_str(&format!(" LIMIT {}", real_limit));
        }

        self.set_sql(Some(sql));

        connection::execute(format!("SET SCHEMA '{schema_name}'").as_str(), [])?;

        Ok(())
    }

    async fn iter_scan_impl(&mut self, row: &mut Row) -> Result<Option<()>> {
        if !connection::has_results() {
            let sql = self
                .get_sql()
                .ok_or_else(|| anyhow!("sql statement was not cached"))?;
            connection::create_arrow(sql.as_str())?;
        }

        if self.get_current_batch().is_none()
            || self.get_current_batch_index()
                >= self
                    .get_current_batch()
                    .as_ref()
                    .ok_or_else(|| anyhow!("current batch not found"))?
                    .num_rows()
        {
            self.set_current_batch_index(0);
            let next_batch = connection::get_next_batch()?;

            if next_batch.is_none() {
                return Ok(None);
            }

            self.set_current_batch(next_batch);
        }

        let current_batch_binding = self.get_current_batch();
        let current_batch = current_batch_binding
            .as_ref()
            .ok_or_else(|| anyhow!("current batch not found"))?;
        let current_batch_index = self.get_current_batch_index();

        for (column_index, target_column) in
            self.get_target_columns().clone().into_iter().enumerate()
        {
            let batch_column = current_batch.column(column_index);
            let cell = batch_column.get_cell(
                current_batch_index,
                target_column.type_oid,
                target_column.type_mod,
            )?;
            row.push(target_column.name.as_str(), cell);
        }

        self.set_current_batch_index(current_batch_index + 1);

        Ok(Some(()))
    }

    fn end_scan_impl(&mut self) {
        connection::clear_arrow();
    }
}

impl From<BaseFdwError> for pg_sys::panic::ErrorReport {
    fn from(value: BaseFdwError) -> Self {
        pg_sys::panic::ErrorReport::new(PgSqlErrorCode::ERRCODE_FDW_ERROR, format!("{}", value), "")
    }
}

#[derive(Error, Debug)]
pub enum BaseFdwError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error(transparent)]
    Options(#[from] OptionsError),
}
