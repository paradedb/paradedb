// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::postgres::datetime::pg_timestamp_micros_to_date32;
use arrow_array::{Array, Date32Array};
use datafusion::arrow::datatypes::{DataType, TimeUnit};
use datafusion::common::Result;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{
    ColumnarValue, ScalarFunctionArgs, ScalarUDF, ScalarUDFImpl, Signature, Volatility,
};
use std::sync::{Arc, LazyLock};

pub const TIMESTAMP_TO_DATE_UDF_NAME: &str = "pdb_timestamp_to_date";

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct TimestampToDateUdf {
    signature: Signature,
}

impl TimestampToDateUdf {
    pub fn new() -> Self {
        let signature = Signature::exact(
            vec![DataType::Timestamp(TimeUnit::Microsecond, None)],
            Volatility::Immutable,
        );

        Self { signature }
    }
}

impl ScalarUDFImpl for TimestampToDateUdf {
    fn name(&self) -> &str {
        TIMESTAMP_TO_DATE_UDF_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Date32)
    }

    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> Result<ColumnarValue> {
        let [arg] = args.args.as_slice() else {
            return Err(DataFusionError::Execution(format!(
                "{} expects exactly one argument, got {}",
                self.name(),
                args.args.len()
            )));
        };

        let input = arg.clone().into_array_of_size(args.number_rows)?;

        let timestamps = input
            .as_any()
            .downcast_ref::<arrow_array::TimestampMicrosecondArray>()
            .ok_or_else(|| {
                DataFusionError::Execution(format!(
                    "{} expects a Timestamp(Microsecond) array, got {}",
                    self.name(),
                    input.data_type()
                ))
            })?;

        let dates: Date32Array = timestamps.unary(pg_timestamp_micros_to_date32);

        Ok(ColumnarValue::Array(Arc::new(dates)))
    }
}

static TIMESTAMP_TO_DATE_UDF: LazyLock<Arc<ScalarUDF>> =
    LazyLock::new(|| Arc::new(ScalarUDF::new_from_impl(TimestampToDateUdf::new())));

pub fn timestamp_to_date_udf() -> Arc<ScalarUDF> {
    Arc::clone(&TIMESTAMP_TO_DATE_UDF)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::TimestampMicrosecondArray;
    use datafusion::arrow::datatypes::Field;
    use datafusion::common::config::ConfigOptions;

    #[test]
    fn converts_timestamp_array_to_date32() {
        let input = TimestampMicrosecondArray::from(vec![
            Some(0),
            Some(-1),
            None,
            Some(i64::MAX),
            Some(i64::MIN),
        ]);

        let number_rows = input.len();

        let args = ScalarFunctionArgs {
            args: vec![ColumnarValue::Array(Arc::new(input))],
            arg_fields: vec![
                Field::new(
                    "timestamp",
                    DataType::Timestamp(TimeUnit::Microsecond, None),
                    true,
                )
                .into(),
            ],
            number_rows,
            return_field: Field::new("date", DataType::Date32, true).into(),
            config_options: Arc::new(ConfigOptions::default()),
        };

        let result = TimestampToDateUdf::new()
            .invoke_with_args(args)
            .expect("timestamp-to-date UDF should succeed");

        let ColumnarValue::Array(result) = result else {
            panic!("timestamp-to-date UDF should return an array");
        };

        let dates = result
            .as_any()
            .downcast_ref::<Date32Array>()
            .expect("timestamp-to-date UDF should return Date32");

        let expected = Date32Array::from(vec![
            Some(10_957),
            Some(10_956),
            None,
            Some(i32::MAX),
            Some(i32::MIN),
        ]);

        assert_eq!(dates, &expected);
    }
}
