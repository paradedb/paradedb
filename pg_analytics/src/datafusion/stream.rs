use async_std::stream::StreamExt;
use async_std::sync::Mutex;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
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
use super::table::PgTableProvider;

const STREAM_ID: &str = "delta_stream";

static STREAM_CACHE: Lazy<Arc<Mutex<HashMap<String, SendableRecordBatchStream>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub struct Stream;

impl Stream {
    pub async fn get_next_batch(
        table_path: &Path,
        schema_name: &str,
        table_name: &str,
    ) -> Result<Option<RecordBatch>, CatalogError> {
        let mut cache = STREAM_CACHE.lock().await;

        let stream = match cache.entry(STREAM_ID.to_string()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(Self::create(table_path, schema_name, table_name).await?),
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
        table_path: &Path,
        schema_name: &str,
        table_name: &str,
    ) -> Result<SendableRecordBatchStream, CatalogError> {
        let table_path = table_path.to_path_buf();
        let schema_name = schema_name.to_string();
        let table_name = table_name.to_string();

        let table_provider = Session::with_tables(&schema_name.clone(), |mut tables| {
            Box::pin(async move {
                let delta_table = tables.get_ref(&table_path).await?;
                PgTableProvider::new(delta_table.clone(), &schema_name, &table_name).await
            })
        })?;

        Ok(table_provider.dataframe().execute_stream().await?)
    }
}
