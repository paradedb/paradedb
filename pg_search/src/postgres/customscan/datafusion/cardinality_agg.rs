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

//! A `cardinality` aggregate for DataFusion that accumulates Tantivy's own HLL
//! sketch. A `pdb.agg()` over a join then reports the same estimate as one over
//! a single table, and the sketch feeds Tantivy's result finalization as-is.

use std::mem::size_of_val;
use std::sync::{Arc, LazyLock};

use arrow_array::cast::AsArray;
use arrow_array::types::{Float64Type, Int64Type, TimestampMicrosecondType, UInt64Type};
use arrow_array::{Array, ArrayRef};
use arrow_schema::{DataType, Field, FieldRef, TimeUnit};
use datafusion::common::ScalarValue;
use datafusion::error::{DataFusionError, Result};
use datafusion::logical_expr::function::{AccumulatorArgs, StateFieldsArgs};
use datafusion::logical_expr::{
    Accumulator, AggregateUDF, AggregateUDFImpl, Signature, Volatility,
};
use datafusion::physical_plan::expressions::Literal;
use tantivy::aggregation::metric::CardinalityCollector;
use tantivy::columnar::{ColumnType, MonotonicallyMappableToU64};

pub const TANTIVY_CARDINALITY_NAME: &str = "tantivy_cardinality";

static TANTIVY_CARDINALITY: LazyLock<Arc<AggregateUDF>> =
    LazyLock::new(|| Arc::new(AggregateUDF::from(TantivyCardinality::new())));

pub fn tantivy_cardinality_udaf() -> Arc<AggregateUDF> {
    Arc::clone(&TANTIVY_CARDINALITY)
}

/// Resolve the UDAF by name, for the plan codecs.
pub fn udaf_by_name(name: &str) -> Option<Arc<AggregateUDF>> {
    (name == TANTIVY_CARDINALITY_NAME).then(tantivy_cardinality_udaf)
}

/// The sketch as it travels between accumulators and to the caller.
pub fn decode_sketch(bytes: &[u8]) -> Result<CardinalityCollector> {
    postcard::from_bytes(bytes)
        .map_err(|e| DataFusionError::Internal(format!("cardinality sketch does not decode: {e}")))
}

fn encode_sketch(collector: &CardinalityCollector) -> Result<Vec<u8>> {
    postcard::to_allocvec(collector)
        .map_err(|e| DataFusionError::Internal(format!("cardinality sketch does not encode: {e}")))
}

/// The column type travels as a plan literal so it survives plan serialization
/// for parallel and MPP execution. It salts numeric values the way segment
/// collection does, so a sketch from here merges with one collected from an
/// index.
fn column_type_from_args(args: &AccumulatorArgs) -> Result<ColumnType> {
    let expr = args.exprs.get(1).ok_or_else(|| {
        DataFusionError::Internal(format!(
            "{TANTIVY_CARDINALITY_NAME} requires a column type argument"
        ))
    })?;
    let literal = expr.as_ref().downcast_ref::<Literal>().ok_or_else(|| {
        DataFusionError::Internal(format!(
            "{TANTIVY_CARDINALITY_NAME} column type must be a literal"
        ))
    })?;
    match literal.value() {
        ScalarValue::UInt8(Some(code)) => ColumnType::try_from_code(*code).map_err(|_| {
            DataFusionError::Internal(format!("unknown tantivy column type code {code}"))
        }),
        other => Err(DataFusionError::Internal(format!(
            "{TANTIVY_CARDINALITY_NAME} column type must be a non-null UInt8 literal, got {other}"
        ))),
    }
}

struct CardinalityAccumulator {
    collector: CardinalityCollector,
}

impl std::fmt::Debug for CardinalityAccumulator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CardinalityAccumulator")
    }
}

impl CardinalityAccumulator {
    /// Registers of an `Hll8` sketch at Tantivy's `LG_K` of 11, the size the
    /// collector holds beyond its own struct.
    const SKETCH_BYTES: usize = 1 << 11;
}

impl Accumulator for CardinalityAccumulator {
    /// Values go in the way Tantivy's segment collection inserts them: terms by
    /// their bytes, numbers in the stored `u64` form of the column.
    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        let array = &values[0];
        match array.data_type() {
            DataType::Utf8 => {
                for v in array.as_string::<i32>().iter().flatten() {
                    self.collector.insert_bytes(v.as_bytes());
                }
            }
            DataType::LargeUtf8 => {
                for v in array.as_string::<i64>().iter().flatten() {
                    self.collector.insert_bytes(v.as_bytes());
                }
            }
            DataType::Utf8View => {
                for v in array.as_string_view().iter().flatten() {
                    self.collector.insert_bytes(v.as_bytes());
                }
            }
            DataType::Binary => {
                for v in array.as_binary::<i32>().iter().flatten() {
                    self.collector.insert_bytes(v);
                }
            }
            DataType::LargeBinary => {
                for v in array.as_binary::<i64>().iter().flatten() {
                    self.collector.insert_bytes(v);
                }
            }
            DataType::BinaryView => {
                for v in array.as_binary_view().iter().flatten() {
                    self.collector.insert_bytes(v);
                }
            }
            DataType::Int64 => {
                for v in array.as_primitive::<Int64Type>().iter().flatten() {
                    self.collector.insert_u64(v.to_u64());
                }
            }
            DataType::Timestamp(TimeUnit::Microsecond, _) => {
                for v in array
                    .as_primitive::<TimestampMicrosecondType>()
                    .iter()
                    .flatten()
                {
                    self.collector.insert_u64(v.to_u64());
                }
            }
            DataType::UInt64 => {
                for v in array.as_primitive::<UInt64Type>().iter().flatten() {
                    self.collector.insert_u64(v);
                }
            }
            DataType::Float64 => {
                for v in array.as_primitive::<Float64Type>().iter().flatten() {
                    self.collector.insert_u64(v.to_u64());
                }
            }
            DataType::Boolean => {
                for v in array.as_boolean().iter().flatten() {
                    self.collector.insert_u64(v.to_u64());
                }
            }
            other => {
                return Err(DataFusionError::NotImplemented(format!(
                    "{TANTIVY_CARDINALITY_NAME} over {other}"
                )));
            }
        }
        Ok(())
    }

    fn evaluate(&mut self) -> Result<ScalarValue> {
        Ok(ScalarValue::Binary(Some(encode_sketch(&self.collector)?)))
    }

    fn size(&self) -> usize {
        size_of_val(self) + Self::SKETCH_BYTES
    }

    fn state(&mut self) -> Result<Vec<ScalarValue>> {
        Ok(vec![self.evaluate()?])
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        for bytes in states[0].as_binary::<i32>().iter().flatten() {
            self.collector
                .merge_fruits(decode_sketch(bytes)?)
                .map_err(|e| DataFusionError::External(Box::new(e)))?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct TantivyCardinality {
    signature: Signature,
}

impl TantivyCardinality {
    fn new() -> Self {
        Self {
            signature: Signature::any(2, Volatility::Immutable),
        }
    }
}

impl AggregateUDFImpl for TantivyCardinality {
    fn name(&self) -> &str {
        TANTIVY_CARDINALITY_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Binary)
    }

    fn accumulator(&self, args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        if args.is_distinct {
            return Err(DataFusionError::NotImplemented(format!(
                "{TANTIVY_CARDINALITY_NAME} does not support DISTINCT"
            )));
        }
        let column_type = column_type_from_args(&args)?;
        Ok(Box::new(CardinalityAccumulator {
            collector: CardinalityCollector::for_column_type(column_type),
        }))
    }

    fn state_fields(&self, args: StateFieldsArgs) -> Result<Vec<FieldRef>> {
        Ok(vec![
            Field::new(format!("{}[sketch]", args.name), DataType::Binary, true).into(),
        ])
    }
}
