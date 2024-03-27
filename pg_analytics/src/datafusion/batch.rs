use deltalake::datafusion::common::arrow::array::{Array, RecordBatch};
use deltalake::datafusion::common::arrow::error::ArrowError;
use std::sync::Arc;
use thiserror::Error;

use super::table::RESERVED_TID_FIELD;

pub trait PostgresBatch {
    fn remove_tid_column(&mut self) -> Result<Arc<dyn Array>, RecordBatchError>;
}

impl PostgresBatch for RecordBatch {
    fn remove_tid_column(&mut self) -> Result<Arc<dyn Array>, RecordBatchError> {
        let schema = self.schema();
        let tid_index = schema.index_of(RESERVED_TID_FIELD)?;

        Ok(self.remove_column(tid_index))
    }
}

#[derive(Error, Debug)]
pub enum RecordBatchError {
    #[error(transparent)]
    Arrow(#[from] ArrowError),
}
