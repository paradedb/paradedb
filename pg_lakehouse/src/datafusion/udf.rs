use datafusion::common::arrow::datatypes::DataType;
use datafusion::common::{exec_err, DataFusionError, ScalarValue};
use datafusion::logical_expr::{ColumnarValue, ScalarUDFImpl, Signature, Volatility};
use std::any::Any;

#[derive(Debug)]
pub struct PgSleep {
    signature: Signature,
}

impl PgSleep {
    pub fn new() -> Self {
        Self {
            signature: Signature::uniform(
                1,
                vec![
                    DataType::Int8,
                    DataType::Int16,
                    DataType::Int32,
                    DataType::Int64,
                    DataType::Float32,
                    DataType::Float64,
                    DataType::UInt8,
                    DataType::UInt16,
                    DataType::UInt32,
                    DataType::UInt64,
                ],
                Volatility::Immutable,
            ),
        }
    }
}

impl ScalarUDFImpl for PgSleep {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn name(&self) -> &str {
        "pg_sleep"
    }
    fn signature(&self) -> &Signature {
        &self.signature
    }
    fn return_type(&self, _args: &[DataType]) -> Result<DataType, DataFusionError> {
        Ok(DataType::Null)
    }
    fn invoke(&self, args: &[ColumnarValue]) -> Result<ColumnarValue, DataFusionError> {
        let sleep_time = match &args[0] {
            ColumnarValue::Scalar(ScalarValue::Int8(Some(v))) => *v as u64,
            ColumnarValue::Scalar(ScalarValue::Int16(Some(v))) => *v as u64,
            ColumnarValue::Scalar(ScalarValue::Int32(Some(v))) => *v as u64,
            ColumnarValue::Scalar(ScalarValue::Int64(Some(v))) => *v as u64,
            ColumnarValue::Scalar(ScalarValue::Float32(Some(v))) => *v as u64,
            ColumnarValue::Scalar(ScalarValue::Float64(Some(v))) => *v as u64,
            ColumnarValue::Scalar(ScalarValue::UInt8(Some(v))) => *v as u64,
            ColumnarValue::Scalar(ScalarValue::UInt16(Some(v))) => *v as u64,
            ColumnarValue::Scalar(ScalarValue::UInt32(Some(v))) => *v as u64,
            ColumnarValue::Scalar(ScalarValue::UInt64(Some(v))) => *v,
            _ => {
                return exec_err!("`pg_sleep` must be called with a non-null scalar Int64");
            }
        };

        std::thread::sleep(std::time::Duration::from_secs(sleep_time));
        Ok(ColumnarValue::Scalar(ScalarValue::Null))
    }
}
