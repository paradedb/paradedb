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

use std::any::Any;
use std::sync::Arc;

use arrow_array::builder::BooleanBuilder;
use arrow_array::{Array, UInt64Array};
use arrow_schema::DataType;
use datafusion::common::{Result, ScalarValue};
use datafusion::logical_expr::{ColumnarValue, ScalarUDFImpl, Signature, Volatility};
use serde::{Deserialize, Serialize};

/// User Defined Function to check if a row's ctid exists in a sorted set of ctids.
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RowInSetUDF {
    set: Arc<Vec<u64>>, // Sorted set of valid ctids
    #[serde(skip, default = "RowInSetUDF::make_signature")]
    signature: Signature,
}

impl RowInSetUDF {
    pub fn new(set: Arc<Vec<u64>>) -> Self {
        Self {
            set,
            signature: Self::make_signature(),
        }
    }

    fn make_signature() -> Signature {
        Signature::exact(vec![DataType::UInt64], Volatility::Immutable)
    }
}

impl ScalarUDFImpl for RowInSetUDF {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        "row_in_set"
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Boolean)
    }

    fn invoke_with_args(
        &self,
        args: datafusion::logical_expr::ScalarFunctionArgs,
    ) -> Result<ColumnarValue> {
        let arg = &args.args[0];
        match arg {
            ColumnarValue::Array(array) => {
                let ctids = array
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .expect("Expected UInt64Array for ctid");
                let mut builder = BooleanBuilder::with_capacity(ctids.len());
                for i in 0..ctids.len() {
                    if ctids.is_null(i) {
                        builder.append_null();
                    } else {
                        let ctid = ctids.value(i);
                        // binary search since set is sorted
                        // TODO: Use Arrow compute kernels (e.g. `in_list` or specialized binary search) for this.
                        builder.append_value(self.set.binary_search(&ctid).is_ok());
                    }
                }
                Ok(ColumnarValue::Array(Arc::new(builder.finish())))
            }
            ColumnarValue::Scalar(scalar) => match scalar {
                ScalarValue::UInt64(Some(ctid)) => {
                    let is_present = self.set.binary_search(ctid).is_ok();
                    Ok(ColumnarValue::Scalar(ScalarValue::Boolean(Some(
                        is_present,
                    ))))
                }
                _ => Ok(ColumnarValue::Scalar(ScalarValue::Boolean(None))),
            },
        }
    }
}
