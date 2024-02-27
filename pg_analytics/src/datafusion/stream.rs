use async_std::stream::StreamExt;
use async_std::task;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::datasource::TableProvider;
use deltalake::datafusion::physical_plan::SendableRecordBatchStream;
use pgrx::*;
use std::collections::{
    hash_map::Entry::{self, Occupied, Vacant},
    HashMap,
};
use std::path::{Path, PathBuf};

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::table::DatafusionTable;
use crate::errors::ParadeError;

pub struct Streams {
    streams: HashMap<PathBuf, SendableRecordBatchStream>,
}

impl Streams {
    pub fn new() -> Result<Self, ParadeError> {
        Ok(Self {
            streams: HashMap::new(),
        })
    }

    pub async fn get_next_batch(
        &mut self,
        pg_relation: &PgRelation,
    ) -> Result<Option<RecordBatch>, ParadeError> {
        let table_path = pg_relation.table_path()?;
        let stream = match Self::get_entry(self, &table_path)? {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(Self::create(pg_relation).await?),
        };

        match stream.next().await {
            Some(Ok(b)) => Ok(Some(b)),
            None => {
                self.streams.remove(&table_path);
                Ok(None)
            }
            Some(Err(err)) => Err(ParadeError::DataFusion(err)),
        }
    }

    async fn create(pg_relation: &PgRelation) -> Result<SendableRecordBatchStream, ParadeError> {
        let schema_name = pg_relation.namespace();
        let table_path = pg_relation.table_path()?;

        let delta_table = DatafusionContext::with_tables(schema_name, |mut tables| {
            task::block_on(tables.get_owned(&table_path))
        })?;

        let (state, task_context) = DatafusionContext::with_session_context(|context| {
            let state = context.state();
            let task_context = context.task_ctx();
            Ok((state, task_context))
        })?;

        Ok(delta_table
            .scan(&state, None, &[], None)
            .await
            .map(|plan| plan.execute(0, task_context))??)
    }

    fn get_entry(
        &mut self,
        table_path: &Path,
    ) -> Result<Entry<PathBuf, SendableRecordBatchStream>, ParadeError> {
        Ok(self.streams.entry(table_path.to_path_buf()))
    }
}
