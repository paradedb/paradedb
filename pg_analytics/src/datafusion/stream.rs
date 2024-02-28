use async_std::stream::StreamExt;
use async_std::task;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::datasource::TableProvider;
use deltalake::datafusion::physical_plan::SendableRecordBatchStream;
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};
use std::path::{Path, PathBuf};

use crate::datafusion::context::DatafusionContext;
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
        schema_name: &str,
        table_path: &Path,
    ) -> Result<Option<RecordBatch>, ParadeError> {
        let stream = match self.streams.entry(table_path.to_path_buf()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(Self::create(schema_name, table_path).await?),
        };

        match stream.next().await {
            Some(Ok(b)) => Ok(Some(b)),
            None => {
                self.streams.remove(table_path);
                Ok(None)
            }
            Some(Err(err)) => Err(ParadeError::DataFusion(err)),
        }
    }

    async fn create(schema_name: &str, table_path: &Path) -> Result<SendableRecordBatchStream, ParadeError> {
        let delta_table = DatafusionContext::with_tables(schema_name, |mut tables| {
            task::block_on(tables.get_owned(table_path))
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
}
