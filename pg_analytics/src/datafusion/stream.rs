use async_std::stream::StreamExt;
use async_std::sync::Mutex;
use deltalake::datafusion::arrow::record_batch::RecordBatch;

use deltalake::datafusion::datasource::TableProvider;
use deltalake::datafusion::physical_plan::SendableRecordBatchStream;
use once_cell::sync::Lazy;
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};
use std::path::Path;
use std::sync::Arc;

use super::catalog::CatalogError;
use super::session::Session;

const STREAM_ID: &str = "delta_stream";

static STREAM_CACHE: Lazy<Arc<Mutex<HashMap<String, SendableRecordBatchStream>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub struct Stream;

impl Stream {
    pub async fn get_next_batch(
        schema_name: &str,
        table_path: &Path,
    ) -> Result<Option<RecordBatch>, CatalogError> {
        let mut cache = STREAM_CACHE.lock().await;

        let stream = match cache.entry(STREAM_ID.to_string()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(Self::create(schema_name, table_path).await?),
        };

        match stream.next().await {
            Some(Ok(b)) => Ok(Some(b)),
            None => {
                cache.remove(STREAM_ID);
                Ok(None)
            }
            Some(Err(err)) => Err(CatalogError::DataFusionError(err)),
        }
    }

    async fn create(
        schema_name: &str,
        table_path: &Path,
    ) -> Result<SendableRecordBatchStream, CatalogError> {
        let table_path = table_path.to_path_buf();
        let delta_table = Session::with_tables(schema_name, |mut tables| {
            Box::pin(async move { Ok(tables.get_owned(&table_path).await?) })
        })?;

        let (state, task_context) = Session::with_session_context(|context| {
            Box::pin(async move {
                let state = context.state();
                let task_context = context.task_ctx();
                Ok((state, task_context))
            })
        })?;

        Ok(delta_table
            .scan(&state, None, &[], None)
            .await
            .map(|plan| plan.execute(0, task_context))??)
    }
}
