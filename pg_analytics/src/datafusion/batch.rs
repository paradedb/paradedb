use deltalake::datafusion::common::arrow::array::{Array, RecordBatch};
use deltalake::datafusion::common::arrow::error::ArrowError;
use std::sync::Arc;

use super::table::{RESERVED_TID_FIELD, RESERVED_XMAX_FIELD, RESERVED_XMIN_FIELD};

pub trait PostgresBatch {
    fn remove_tid_column(&mut self) -> Result<Arc<dyn Array>, ArrowError>;
    fn remove_xmin_column(&mut self) -> Result<Arc<dyn Array>, ArrowError>;
    fn remove_xmax_column(&mut self) -> Result<Arc<dyn Array>, ArrowError>;
}

impl PostgresBatch for RecordBatch {
    fn remove_tid_column(&mut self) -> Result<Arc<dyn Array>, ArrowError> {
        let schema = self.schema();
        let index = schema.index_of(RESERVED_TID_FIELD)?;

        Ok(self.remove_column(index))
    }

    fn remove_xmin_column(&mut self) -> Result<Arc<dyn Array>, ArrowError> {
        let schema = self.schema();
        let index = schema.index_of(RESERVED_XMIN_FIELD)?;

        Ok(self.remove_column(index))
    }

    fn remove_xmax_column(&mut self) -> Result<Arc<dyn Array>, ArrowError> {
        let schema = self.schema();
        let index = schema.index_of(RESERVED_XMAX_FIELD)?;

        Ok(self.remove_column(index))
    }
}
