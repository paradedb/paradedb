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

//! Aggregate UDAFs for NUMERIC columns.
//!
//! DataFusion's native `sum`/`avg` cannot be used for NUMERIC fast fields:
//!
//! - `Numeric64` columns are scaled `Int64` values with sentinel encodings for
//!   NaN and +/-Infinity, so a native `sum(Int64)` would add sentinels into the
//!   total and can overflow i64.
//! - `NumericBytes` columns are `decimal-bytes` encoded `BinaryView` values,
//!   which native aggregates do not accept at all.
//!
//! The UDAFs here accumulate exactly (i128 for `Numeric64`, `BigDecimal` for
//! `NumericBytes`) and emit a `decimal-bytes` encoded `Binary`. That encoding
//! represents NaN and +/-Infinity natively and is what the Arrow-to-Datum
//! conversion already decodes, so partial states, MPP merges, and final
//! results all use one representation.
//!
//! AVG returns `[count: u64 BE, sum: decimal-bytes]` in one `Binary` value.
//! The final `sum / count` division happens on the Postgres side with
//! `AnyNumeric` so the result scale matches Postgres' division rules.

use std::str::FromStr;
use std::sync::{Arc, LazyLock};

use arrow_array::cast::AsArray;
use arrow_array::{Array, ArrayRef};
use arrow_schema::{DataType, Field, FieldRef};
use bigdecimal::BigDecimal;
use bigdecimal::num_bigint::BigInt;
use datafusion::common::ScalarValue;
use datafusion::error::{DataFusionError, Result};
use datafusion::logical_expr::function::{AccumulatorArgs, StateFieldsArgs};
use datafusion::logical_expr::{
    Accumulator, AggregateUDF, AggregateUDFImpl, Signature, Volatility,
};
use datafusion::physical_plan::expressions::Literal;
use decimal_bytes::{Decimal, Decimal64NoScale};

pub const NUMERIC64_SUM_NAME: &str = "numeric64_sum";
pub const NUMERIC64_AVG_NAME: &str = "numeric64_avg";
pub const NUMERIC_BYTES_SUM_NAME: &str = "numeric_bytes_sum";
pub const NUMERIC_BYTES_AVG_NAME: &str = "numeric_bytes_avg";

static NUMERIC64_SUM: LazyLock<Arc<AggregateUDF>> =
    LazyLock::new(|| Arc::new(AggregateUDF::from(Numeric64Sum::new())));
static NUMERIC64_AVG: LazyLock<Arc<AggregateUDF>> =
    LazyLock::new(|| Arc::new(AggregateUDF::from(Numeric64Avg::new())));
static NUMERIC_BYTES_SUM: LazyLock<Arc<AggregateUDF>> =
    LazyLock::new(|| Arc::new(AggregateUDF::from(NumericBytesSum::new())));
static NUMERIC_BYTES_AVG: LazyLock<Arc<AggregateUDF>> =
    LazyLock::new(|| Arc::new(AggregateUDF::from(NumericBytesAvg::new())));

pub fn numeric64_sum_udaf() -> Arc<AggregateUDF> {
    Arc::clone(&NUMERIC64_SUM)
}

pub fn numeric64_avg_udaf() -> Arc<AggregateUDF> {
    Arc::clone(&NUMERIC64_AVG)
}

pub fn numeric_bytes_sum_udaf() -> Arc<AggregateUDF> {
    Arc::clone(&NUMERIC_BYTES_SUM)
}

pub fn numeric_bytes_avg_udaf() -> Arc<AggregateUDF> {
    Arc::clone(&NUMERIC_BYTES_AVG)
}

/// Resolve a numeric aggregate UDAF by name, for the plan codecs. These
/// functions are not in any session registry, so serialized plans (parallel
/// and MPP dispatch) decode them through here.
pub fn udaf_by_name(name: &str) -> Option<Arc<AggregateUDF>> {
    match name {
        NUMERIC64_SUM_NAME => Some(numeric64_sum_udaf()),
        NUMERIC64_AVG_NAME => Some(numeric64_avg_udaf()),
        NUMERIC_BYTES_SUM_NAME => Some(numeric_bytes_sum_udaf()),
        NUMERIC_BYTES_AVG_NAME => Some(numeric_bytes_avg_udaf()),
        _ => None,
    }
}

/// Split an AVG result blob into `(count, decimal-bytes sum)`.
pub fn decode_avg_blob(blob: &[u8]) -> Result<(u64, &[u8])> {
    if blob.len() < 8 {
        return Err(DataFusionError::Internal(format!(
            "numeric AVG blob too short: {} bytes",
            blob.len()
        )));
    }
    let count = u64::from_be_bytes(blob[..8].try_into().unwrap());
    Ok((count, &blob[8..]))
}

/// Postgres SUM semantics for NUMERIC special values: NaN absorbs everything,
/// and +Infinity combined with -Infinity yields NaN. These rules are
/// associative and commutative, so partial-aggregate merges preserve them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Special {
    Finite,
    PosInf,
    NegInf,
    Nan,
}

impl Special {
    fn absorb(&mut self, other: Special) {
        use Special::*;
        *self = match (*self, other) {
            (Nan, _) | (_, Nan) => Nan,
            (PosInf, NegInf) | (NegInf, PosInf) => Nan,
            (PosInf, _) | (_, PosInf) => PosInf,
            (NegInf, _) | (_, NegInf) => NegInf,
            (Finite, Finite) => Finite,
        };
    }
}

/// Exact running sum shared by all four accumulators.
///
/// `Numeric64` updates accumulate into `scaled`, an i128 of scaled values,
/// which cannot overflow (2^63 rows of i64 would be needed). Merged partial
/// states and `NumericBytes` updates accumulate into `decimal`.
#[derive(Debug)]
struct NumericSumState {
    seen: bool,
    special: Special,
    scaled: i128,
    decimal: BigDecimal,
}

impl NumericSumState {
    fn new() -> Self {
        Self {
            seen: false,
            special: Special::Finite,
            scaled: 0,
            decimal: BigDecimal::from(0),
        }
    }

    fn add_scaled_i64(&mut self, value: i64) {
        self.seen = true;
        let v = Decimal64NoScale::from_raw(value);
        if v.is_nan() {
            self.special.absorb(Special::Nan);
        } else if v.is_pos_infinity() {
            self.special.absorb(Special::PosInf);
        } else if v.is_neg_infinity() {
            self.special.absorb(Special::NegInf);
        } else if self.special == Special::Finite {
            self.scaled += value as i128;
        }
    }

    fn add_decimal_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        let d = Decimal::from_bytes(bytes).map_err(|e| {
            DataFusionError::Internal(format!("failed to decode decimal bytes: {e:?}"))
        })?;
        self.seen = true;
        if d.is_nan() {
            self.special.absorb(Special::Nan);
        } else if d.is_pos_infinity() {
            self.special.absorb(Special::PosInf);
        } else if d.is_neg_infinity() {
            self.special.absorb(Special::NegInf);
        } else if self.special == Special::Finite {
            let parsed = BigDecimal::from_str(&d.to_string()).map_err(|e| {
                DataFusionError::Internal(format!("failed to parse decimal '{d}': {e}"))
            })?;
            self.decimal += parsed;
        }
        Ok(())
    }

    /// Encode the running sum as decimal-bytes. `None` when no rows were seen,
    /// which surfaces as SQL NULL (Postgres `SUM` over zero rows).
    fn encode(&self, scale: i32) -> Result<Option<Vec<u8>>> {
        if !self.seen {
            return Ok(None);
        }
        let d = match self.special {
            Special::Nan => Decimal::nan(),
            Special::PosInf => Decimal::infinity(),
            Special::NegInf => Decimal::neg_infinity(),
            Special::Finite => {
                let mut total = self.decimal.clone();
                if self.scaled != 0 {
                    total += BigDecimal::new(BigInt::from(self.scaled), scale as i64);
                }
                Decimal::from_str(&total.to_string()).map_err(|e| {
                    DataFusionError::Internal(format!(
                        "failed to encode numeric sum '{total}': {e:?}"
                    ))
                })?
            }
        };
        Ok(Some(d.into_bytes()))
    }
}

/// Add every non-null decimal-bytes value of `array` into `state`.
/// Accepts both the scan's `BinaryView` columns and the `Binary` state arrays.
fn accumulate_binary_array(state: &mut NumericSumState, array: &ArrayRef) -> Result<u64> {
    let mut non_null = 0u64;
    match array.data_type() {
        DataType::BinaryView => {
            let arr = array.as_binary_view();
            for v in arr.iter().flatten() {
                state.add_decimal_bytes(v)?;
                non_null += 1;
            }
        }
        DataType::Binary => {
            let arr = array.as_binary::<i32>();
            for v in arr.iter().flatten() {
                state.add_decimal_bytes(v)?;
                non_null += 1;
            }
        }
        other => {
            return Err(DataFusionError::Internal(format!(
                "numeric aggregate expected a binary array, got {other}"
            )));
        }
    }
    Ok(non_null)
}

/// Extract the constant scale argument (second UDAF argument) at accumulator
/// creation time. The scale travels as a plan literal so it survives
/// serialization for parallel and MPP execution.
fn scale_from_args(args: &AccumulatorArgs, name: &str) -> Result<i32> {
    let expr = args
        .exprs
        .get(1)
        .ok_or_else(|| DataFusionError::Internal(format!("{name} requires a scale argument")))?;
    let literal = expr
        .as_ref()
        .downcast_ref::<Literal>()
        .ok_or_else(|| DataFusionError::Internal(format!("{name} scale must be a literal")))?;
    match literal.value() {
        ScalarValue::Int32(Some(scale)) => Ok(*scale),
        other => Err(DataFusionError::Internal(format!(
            "{name} scale must be a non-null Int32 literal, got {other}"
        ))),
    }
}

fn reject_distinct(args: &AccumulatorArgs, name: &str) -> Result<()> {
    if args.is_distinct {
        return Err(DataFusionError::NotImplemented(format!(
            "{name} does not support DISTINCT"
        )));
    }
    Ok(())
}

fn sum_state_fields(args: StateFieldsArgs) -> Vec<FieldRef> {
    vec![Field::new(format!("{}[sum]", args.name), DataType::Binary, true).into()]
}

fn avg_state_fields(args: StateFieldsArgs) -> Vec<FieldRef> {
    vec![
        Field::new(format!("{}[sum]", args.name), DataType::Binary, true).into(),
        Field::new(format!("{}[count]", args.name), DataType::UInt64, true).into(),
    ]
}

// ============================================================================
// SUM
// ============================================================================

#[derive(Debug)]
struct NumericSumAccumulator {
    /// Scale of the scaled `Int64` input column; 0 for decimal-bytes input,
    /// whose values are self-describing.
    scale: i32,
    state: NumericSumState,
}

impl NumericSumAccumulator {
    fn new(scale: i32) -> Self {
        Self {
            scale,
            state: NumericSumState::new(),
        }
    }

    fn update_binary(&mut self, values: &[ArrayRef]) -> Result<()> {
        accumulate_binary_array(&mut self.state, &values[0])?;
        Ok(())
    }

    fn update_scaled_i64(&mut self, values: &[ArrayRef]) -> Result<()> {
        let arr = values[0].as_primitive::<arrow_array::types::Int64Type>();
        for v in arr.iter().flatten() {
            self.state.add_scaled_i64(v);
        }
        Ok(())
    }
}

impl Accumulator for NumericSumAccumulator {
    fn update_batch(&mut self, _values: &[ArrayRef]) -> Result<()> {
        unreachable!("update_batch is provided by the concrete wrapper")
    }

    fn evaluate(&mut self) -> Result<ScalarValue> {
        Ok(ScalarValue::Binary(self.state.encode(self.scale)?))
    }

    fn size(&self) -> usize {
        size_of_val(self)
    }

    fn state(&mut self) -> Result<Vec<ScalarValue>> {
        Ok(vec![self.evaluate()?])
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        accumulate_binary_array(&mut self.state, &states[0])?;
        Ok(())
    }
}

/// The concrete accumulators differ only in how a batch of input values is
/// folded into the shared inner state.
macro_rules! delegate_accumulator {
    ($name:ident, $inner:ident, $update:ident) => {
        #[derive(Debug)]
        struct $name($inner);

        impl Accumulator for $name {
            fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
                self.0.$update(values)
            }

            fn evaluate(&mut self) -> Result<ScalarValue> {
                self.0.evaluate()
            }

            fn size(&self) -> usize {
                self.0.size()
            }

            fn state(&mut self) -> Result<Vec<ScalarValue>> {
                self.0.state()
            }

            fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
                self.0.merge_batch(states)
            }
        }
    };
}

delegate_accumulator!(
    Numeric64SumAccumulator,
    NumericSumAccumulator,
    update_scaled_i64
);
delegate_accumulator!(
    NumericBytesSumAccumulator,
    NumericSumAccumulator,
    update_binary
);

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Numeric64Sum {
    signature: Signature,
}

impl Numeric64Sum {
    fn new() -> Self {
        Self {
            signature: Signature::exact(
                vec![DataType::Int64, DataType::Int32],
                Volatility::Immutable,
            ),
        }
    }
}

impl AggregateUDFImpl for Numeric64Sum {
    fn name(&self) -> &str {
        NUMERIC64_SUM_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Binary)
    }

    fn accumulator(&self, args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        reject_distinct(&args, self.name())?;
        let scale = scale_from_args(&args, self.name())?;
        Ok(Box::new(Numeric64SumAccumulator(
            NumericSumAccumulator::new(scale),
        )))
    }

    fn state_fields(&self, args: StateFieldsArgs) -> Result<Vec<FieldRef>> {
        Ok(sum_state_fields(args))
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct NumericBytesSum {
    signature: Signature,
}

impl NumericBytesSum {
    fn new() -> Self {
        Self {
            signature: Signature::exact(vec![DataType::BinaryView], Volatility::Immutable),
        }
    }
}

impl AggregateUDFImpl for NumericBytesSum {
    fn name(&self) -> &str {
        NUMERIC_BYTES_SUM_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Binary)
    }

    fn accumulator(&self, args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        reject_distinct(&args, self.name())?;
        Ok(Box::new(NumericBytesSumAccumulator(
            NumericSumAccumulator::new(0),
        )))
    }

    fn state_fields(&self, args: StateFieldsArgs) -> Result<Vec<FieldRef>> {
        Ok(sum_state_fields(args))
    }
}

// ============================================================================
// AVG
// ============================================================================

#[derive(Debug)]
struct NumericAvgAccumulator {
    scale: i32,
    state: NumericSumState,
    count: u64,
}

impl NumericAvgAccumulator {
    fn new(scale: i32) -> Self {
        Self {
            scale,
            state: NumericSumState::new(),
            count: 0,
        }
    }

    fn update_binary(&mut self, values: &[ArrayRef]) -> Result<()> {
        self.count += accumulate_binary_array(&mut self.state, &values[0])?;
        Ok(())
    }

    fn update_scaled_i64(&mut self, values: &[ArrayRef]) -> Result<()> {
        let arr = values[0].as_primitive::<arrow_array::types::Int64Type>();
        for v in arr.iter().flatten() {
            self.state.add_scaled_i64(v);
            self.count += 1;
        }
        Ok(())
    }

    fn sum_bytes(&self) -> Result<Option<Vec<u8>>> {
        self.state.encode(self.scale)
    }
}

impl Accumulator for NumericAvgAccumulator {
    fn update_batch(&mut self, _values: &[ArrayRef]) -> Result<()> {
        unreachable!("update_batch is provided by the concrete wrapper")
    }

    fn evaluate(&mut self) -> Result<ScalarValue> {
        let blob = self.sum_bytes()?.map(|sum| {
            let mut blob = Vec::with_capacity(8 + sum.len());
            blob.extend_from_slice(&self.count.to_be_bytes());
            blob.extend_from_slice(&sum);
            blob
        });
        Ok(ScalarValue::Binary(blob))
    }

    fn size(&self) -> usize {
        size_of_val(self)
    }

    fn state(&mut self) -> Result<Vec<ScalarValue>> {
        Ok(vec![
            ScalarValue::Binary(self.sum_bytes()?),
            ScalarValue::UInt64(Some(self.count)),
        ])
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        accumulate_binary_array(&mut self.state, &states[0])?;
        let counts = states[1].as_primitive::<arrow_array::types::UInt64Type>();
        for c in counts.iter().flatten() {
            self.count += c;
        }
        Ok(())
    }
}

delegate_accumulator!(
    Numeric64AvgAccumulator,
    NumericAvgAccumulator,
    update_scaled_i64
);
delegate_accumulator!(
    NumericBytesAvgAccumulator,
    NumericAvgAccumulator,
    update_binary
);

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Numeric64Avg {
    signature: Signature,
}

impl Numeric64Avg {
    fn new() -> Self {
        Self {
            signature: Signature::exact(
                vec![DataType::Int64, DataType::Int32],
                Volatility::Immutable,
            ),
        }
    }
}

impl AggregateUDFImpl for Numeric64Avg {
    fn name(&self) -> &str {
        NUMERIC64_AVG_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Binary)
    }

    fn accumulator(&self, args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        reject_distinct(&args, self.name())?;
        let scale = scale_from_args(&args, self.name())?;
        Ok(Box::new(Numeric64AvgAccumulator(
            NumericAvgAccumulator::new(scale),
        )))
    }

    fn state_fields(&self, args: StateFieldsArgs) -> Result<Vec<FieldRef>> {
        Ok(avg_state_fields(args))
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct NumericBytesAvg {
    signature: Signature,
}

impl NumericBytesAvg {
    fn new() -> Self {
        Self {
            signature: Signature::exact(vec![DataType::BinaryView], Volatility::Immutable),
        }
    }
}

impl AggregateUDFImpl for NumericBytesAvg {
    fn name(&self) -> &str {
        NUMERIC_BYTES_AVG_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Binary)
    }

    fn accumulator(&self, args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        reject_distinct(&args, self.name())?;
        Ok(Box::new(NumericBytesAvgAccumulator(
            NumericAvgAccumulator::new(0),
        )))
    }

    fn state_fields(&self, args: StateFieldsArgs) -> Result<Vec<FieldRef>> {
        Ok(avg_state_fields(args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes_of(s: &str) -> Vec<u8> {
        Decimal::from_str(s).unwrap().into_bytes()
    }

    fn decode(bytes: &[u8]) -> String {
        Decimal::from_bytes(bytes).unwrap().to_string()
    }

    #[test]
    fn sum_bytes_exact_at_78_digits() {
        let mut state = NumericSumState::new();
        let nines = "9".repeat(78);
        state.add_decimal_bytes(&bytes_of(&nines)).unwrap();
        state.add_decimal_bytes(&bytes_of("1")).unwrap();
        let out = state.encode(0).unwrap().unwrap();
        let mut expected = String::from("1");
        expected.push_str(&"0".repeat(78));
        assert_eq!(decode(&out), expected);
    }

    #[test]
    fn sum_scaled_i64_uses_scale() {
        let mut state = NumericSumState::new();
        state.add_scaled_i64(110);
        state.add_scaled_i64(220);
        let out = state.encode(2).unwrap().unwrap();
        assert_eq!(decode(&out), "3.3");
    }

    #[test]
    fn sum_mixes_scaled_and_merged_decimal() {
        let mut state = NumericSumState::new();
        state.add_scaled_i64(150); // 1.50 at scale 2
        state.add_decimal_bytes(&bytes_of("2.25")).unwrap();
        let out = state.encode(2).unwrap().unwrap();
        assert_eq!(decode(&out), "3.75");
    }

    #[test]
    fn sum_special_values_follow_postgres() {
        // NaN absorbs everything
        let mut state = NumericSumState::new();
        state.add_decimal_bytes(&bytes_of("1")).unwrap();
        state
            .add_decimal_bytes(&Decimal::nan().into_bytes())
            .unwrap();
        assert_eq!(decode(&state.encode(0).unwrap().unwrap()), "NaN");

        // +Infinity + -Infinity = NaN
        let mut state = NumericSumState::new();
        state
            .add_decimal_bytes(&Decimal::infinity().into_bytes())
            .unwrap();
        state
            .add_decimal_bytes(&Decimal::neg_infinity().into_bytes())
            .unwrap();
        assert_eq!(decode(&state.encode(0).unwrap().unwrap()), "NaN");

        // +Infinity + finite = +Infinity
        let mut state = NumericSumState::new();
        state
            .add_decimal_bytes(&Decimal::infinity().into_bytes())
            .unwrap();
        state.add_decimal_bytes(&bytes_of("42")).unwrap();
        assert_eq!(decode(&state.encode(0).unwrap().unwrap()), "Infinity");
    }

    #[test]
    fn sum_i64_sentinels() {
        let mut state = NumericSumState::new();
        state.add_scaled_i64(Decimal64NoScale::nan().raw());
        state.add_scaled_i64(100);
        assert_eq!(decode(&state.encode(2).unwrap().unwrap()), "NaN");
    }

    #[test]
    fn empty_state_encodes_null() {
        let state = NumericSumState::new();
        assert!(state.encode(0).unwrap().is_none());
    }

    #[test]
    fn avg_blob_roundtrip() {
        let sum = bytes_of("10.5");
        let mut blob = 3u64.to_be_bytes().to_vec();
        blob.extend_from_slice(&sum);
        let (count, sum_bytes) = decode_avg_blob(&blob).unwrap();
        assert_eq!(count, 3);
        assert_eq!(decode(sum_bytes), "10.5");
    }
}
