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

//! Vectorized integer `+`, `-` and `*` with PostgreSQL overflow semantics.
//!
//! DataFusion's planner builds `BinaryExpr` with wrapping integer kernels, so
//! `2147483647 + 1` silently becomes `-2147483648` where PostgreSQL raises
//! `integer out of range`. These UDFs use Arrow's checked kernels instead and
//! raise PostgreSQL's error on overflow.
//!
//! Both arguments must already have the operator's result type. PostgreSQL's
//! cross-width operators (`int24pl`, `int48mul`, ...) widen the narrower operand
//! and compute at the result width, so the translator casts both operands to the
//! Arrow type of `opresulttype` and the overflow check runs at that width.

use std::convert::Infallible;
use std::sync::{Arc, LazyLock};

use arrow_array::{ArrayRef, Datum};
use datafusion::arrow::compute::kernels::numeric;
use datafusion::arrow::datatypes::DataType;
use datafusion::arrow::error::ArrowError;
use datafusion::common::{Result, ScalarValue};
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{
    ColumnarValue, ScalarFunctionArgs, ScalarUDF, ScalarUDFImpl, Signature, TypeSignature,
    Volatility,
};
use pgrx::{PgSqlErrorCode, pg_sys};

pub const INT_ADD_UDF_NAME: &str = "pdb_int_add";
pub const INT_SUB_UDF_NAME: &str = "pdb_int_sub";
pub const INT_MUL_UDF_NAME: &str = "pdb_int_mul";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntOp {
    Add,
    Sub,
    Mul,
}

impl IntOp {
    fn name(self) -> &'static str {
        match self {
            IntOp::Add => INT_ADD_UDF_NAME,
            IntOp::Sub => INT_SUB_UDF_NAME,
            IntOp::Mul => INT_MUL_UDF_NAME,
        }
    }

    /// Arrow's `add`, `sub` and `mul` return `ArithmeticOverflow` for integers;
    /// the `*_wrapping` variants are what `BinaryExpr` uses.
    fn apply(self, a: &dyn Datum, b: &dyn Datum) -> Result<ArrayRef, ArrowError> {
        match self {
            IntOp::Add => numeric::add(a, b),
            IntOp::Sub => numeric::sub(a, b),
            IntOp::Mul => numeric::mul(a, b),
        }
    }
}

/// PostgreSQL's overflow message at the given result width.
fn out_of_range_message(ty: &DataType) -> Option<&'static str> {
    match ty {
        DataType::Int16 => Some("smallint out of range"),
        DataType::Int32 => Some("integer out of range"),
        DataType::Int64 => Some("bigint out of range"),
        _ => None,
    }
}

fn compute(op: IntOp, a: &ColumnarValue, b: &ColumnarValue) -> Result<ColumnarValue> {
    let ty = a.data_type();
    if ty != b.data_type() || out_of_range_message(&ty).is_none() {
        return Err(DataFusionError::Internal(format!(
            "{} expects two Int16, Int32 or Int64 arguments of the same type, got ({ty}, {})",
            op.name(),
            b.data_type()
        )));
    }

    let result = match (a, b) {
        (ColumnarValue::Array(a), ColumnarValue::Array(b)) => op.apply(a, b),
        (ColumnarValue::Scalar(a), ColumnarValue::Array(b)) => op.apply(&a.to_scalar()?, b),
        (ColumnarValue::Array(a), ColumnarValue::Scalar(b)) => op.apply(a, &b.to_scalar()?),
        (ColumnarValue::Scalar(a), ColumnarValue::Scalar(b)) => {
            let array = op
                .apply(&a.to_scalar()?, &b.to_scalar()?)
                .map_err(|err| raise(err, &ty))?;
            return Ok(ColumnarValue::Scalar(ScalarValue::try_from_array(
                array.as_ref(),
                0,
            )?));
        }
    };
    result
        .map(ColumnarValue::Array)
        .map_err(|err| raise(err, &ty))
}

/// Raise PostgreSQL's overflow error on the main thread, so the leader reports
/// SQLSTATE 22003. MPP workers get it back as an execution error.
fn raise(err: ArrowError, ty: &DataType) -> DataFusionError {
    let ArrowError::ArithmeticOverflow(_) = err else {
        return err.into();
    };
    let Some(msg) = out_of_range_message(ty) else {
        return err.into();
    };
    match crate::postgres::customscan::mpp::try_run_on_main_thread(move |_| -> Result<Infallible> {
        pg_sys::ereport!(
            ERROR,
            PgSqlErrorCode::ERRCODE_NUMERIC_VALUE_OUT_OF_RANGE,
            msg
        );
    }) {
        Ok(never) => match never {},
        Err(e) => e,
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct IntArithUdf {
    op: IntOp,
    signature: Signature,
}

impl ScalarUDFImpl for IntArithUdf {
    fn name(&self) -> &str {
        self.op.name()
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, arg_types: &[DataType]) -> Result<DataType> {
        Ok(arg_types[0].clone())
    }

    fn invoke_with_args(&self, args: ScalarFunctionArgs) -> Result<ColumnarValue> {
        let [a, b] = args.args.as_slice() else {
            return Err(DataFusionError::Internal(format!(
                "{} expects two arguments, got {}",
                self.op.name(),
                args.args.len()
            )));
        };
        compute(self.op, a, b)
    }
}

fn new_udf(op: IntOp) -> Arc<ScalarUDF> {
    let exact = |t: DataType| TypeSignature::Exact(vec![t.clone(), t]);
    Arc::new(ScalarUDF::new_from_impl(IntArithUdf {
        op,
        signature: Signature::one_of(
            vec![
                exact(DataType::Int16),
                exact(DataType::Int32),
                exact(DataType::Int64),
            ],
            Volatility::Immutable,
        ),
    }))
}

static INT_ADD_UDF: LazyLock<Arc<ScalarUDF>> = LazyLock::new(|| new_udf(IntOp::Add));
static INT_SUB_UDF: LazyLock<Arc<ScalarUDF>> = LazyLock::new(|| new_udf(IntOp::Sub));
static INT_MUL_UDF: LazyLock<Arc<ScalarUDF>> = LazyLock::new(|| new_udf(IntOp::Mul));

pub fn int_arith_udf(op: IntOp) -> Arc<ScalarUDF> {
    Arc::clone(match op {
        IntOp::Add => &INT_ADD_UDF,
        IntOp::Sub => &INT_SUB_UDF,
        IntOp::Mul => &INT_MUL_UDF,
    })
}

pub fn int_arith_udfs() -> [Arc<ScalarUDF>; 3] {
    [
        int_arith_udf(IntOp::Add),
        int_arith_udf(IntOp::Sub),
        int_arith_udf(IntOp::Mul),
    ]
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use arrow_array::cast::AsArray;
    use arrow_array::types::{Int16Type, Int32Type, Int64Type};
    use arrow_array::{Array, Int16Array, Int32Array, Int64Array};
    use pgrx::prelude::*;

    fn array(a: impl Array + 'static) -> ColumnarValue {
        ColumnarValue::Array(Arc::new(a))
    }

    fn scalar_i64(v: i64) -> ColumnarValue {
        ColumnarValue::Scalar(ScalarValue::Int64(Some(v)))
    }

    fn is_overflow(result: Result<ArrayRef, ArrowError>) -> bool {
        matches!(result, Err(ArrowError::ArithmeticOverflow(_)))
    }

    #[pg_test]
    fn checked_at_each_width() {
        let i16s = Int16Array::from(vec![i16::MAX, i16::MIN, 1]);
        let i32s = Int32Array::from(vec![i32::MAX, i32::MIN, 1]);
        let i64s = Int64Array::from(vec![i64::MAX, i64::MIN, 1]);
        for (op, a, b) in [
            (IntOp::Add, 0usize, 2usize), // MAX + 1
            (IntOp::Sub, 1, 2),           // MIN - 1
            (IntOp::Mul, 0, 0),           // MAX * MAX
        ] {
            let pick16 = |i: usize| Int16Array::from(vec![i16s.value(i)]);
            let pick32 = |i: usize| Int32Array::from(vec![i32s.value(i)]);
            let pick64 = |i: usize| Int64Array::from(vec![i64s.value(i)]);
            assert!(is_overflow(op.apply(&pick16(a), &pick16(b))), "{op:?} i16");
            assert!(is_overflow(op.apply(&pick32(a), &pick32(b))), "{op:?} i32");
            assert!(is_overflow(op.apply(&pick64(a), &pick64(b))), "{op:?} i64");
        }

        // MIN * -1 overflows at every width.
        assert!(is_overflow(IntOp::Mul.apply(
            &Int16Array::from(vec![i16::MIN]),
            &Int16Array::from(vec![-1i16])
        )));
        assert!(is_overflow(IntOp::Mul.apply(
            &Int32Array::from(vec![i32::MIN]),
            &Int32Array::from(vec![-1])
        )));
        assert!(is_overflow(IntOp::Mul.apply(
            &Int64Array::from(vec![i64::MIN]),
            &Int64Array::from(vec![-1i64])
        )));
    }

    #[pg_test]
    fn boundaries_that_fit() {
        let a = Int32Array::from(vec![i32::MAX - 1, i32::MIN + 1, i32::MIN, -46340, 46340]);
        let b = Int32Array::from(vec![1, -1, 0, 46341, 46340]);
        let sum = IntOp::Add.apply(&a, &b).unwrap();
        assert_eq!(
            sum.as_primitive::<Int32Type>(),
            &Int32Array::from(vec![i32::MAX, i32::MIN, i32::MIN, 1, 92680])
        );
        let diff = IntOp::Sub.apply(&a, &b).unwrap();
        assert_eq!(
            diff.as_primitive::<Int32Type>(),
            &Int32Array::from(vec![i32::MAX - 2, i32::MIN + 2, i32::MIN, -92681, 0])
        );
        let product = IntOp::Mul.apply(&a, &b).unwrap();
        assert_eq!(
            product.as_primitive::<Int32Type>(),
            &Int32Array::from(vec![i32::MAX - 1, i32::MAX, 0, -2147441940, 2147395600])
        );
    }

    #[pg_test]
    fn boundaries_that_fit_i16_i64() {
        let product = IntOp::Mul
            .apply(
                &Int16Array::from(vec![181, -128, i16::MIN]),
                &Int16Array::from(vec![181, 256, 1]),
            )
            .unwrap();
        assert_eq!(
            product.as_primitive::<Int16Type>(),
            &Int16Array::from(vec![32761, i16::MIN, i16::MIN])
        );
        let diff = IntOp::Sub
            .apply(
                &Int64Array::from(vec![-1, 0]),
                &Int64Array::from(vec![i64::MAX, i64::MIN + 1]),
            )
            .unwrap();
        assert_eq!(
            diff.as_primitive::<Int64Type>(),
            &Int64Array::from(vec![i64::MIN, i64::MAX])
        );
    }

    #[pg_test]
    fn nulls_propagate_and_hide_overflow() {
        // The value under a NULL slot is not checked, so MAX + NULL is not an overflow.
        let a = array(Int32Array::from(vec![Some(i32::MAX), Some(2), None]));
        let b = array(Int32Array::from(vec![None, Some(3), Some(i32::MAX)]));
        let ColumnarValue::Array(out) = compute(IntOp::Add, &a, &b).unwrap() else {
            panic!("expected an array");
        };
        assert_eq!(
            out.as_primitive::<Int32Type>(),
            &Int32Array::from(vec![None, Some(5), None])
        );

        let null = ColumnarValue::Scalar(ScalarValue::Int64(None));
        let b = array(Int64Array::from(vec![1, i64::MAX]));
        let ColumnarValue::Array(out) = compute(IntOp::Mul, &null, &b).unwrap() else {
            panic!("expected an array");
        };
        assert_eq!(out.null_count(), 2);
    }

    #[pg_test]
    fn scalars_and_empty_arrays() {
        let a = ColumnarValue::Scalar(ScalarValue::Int16(Some(7)));
        let b = ColumnarValue::Scalar(ScalarValue::Int16(Some(-3)));
        let ColumnarValue::Scalar(out) = compute(IntOp::Mul, &a, &b).unwrap() else {
            panic!("expected a scalar");
        };
        assert_eq!(out, ScalarValue::Int16(Some(-21)));

        let col = array(Int64Array::from(vec![10, 20]));
        let ColumnarValue::Array(out) = compute(IntOp::Sub, &col, &scalar_i64(5)).unwrap() else {
            panic!("expected an array");
        };
        assert_eq!(
            out.as_primitive::<Int64Type>(),
            &Int64Array::from(vec![5, 15])
        );

        let empty = array(Int32Array::from(Vec::<i32>::new()));
        let ColumnarValue::Array(out) = compute(IntOp::Add, &empty, &empty).unwrap() else {
            panic!("expected an array");
        };
        assert_eq!(out.len(), 0);
    }

    #[pg_test]
    fn rejects_mixed_or_non_integer_types() {
        let i32s = array(Int32Array::from(vec![1]));
        let i64s = array(Int64Array::from(vec![1]));
        assert!(matches!(
            compute(IntOp::Add, &i32s, &i64s),
            Err(DataFusionError::Internal(_))
        ));
        let f64s = array(arrow_array::Float64Array::from(vec![1.0]));
        assert!(matches!(
            compute(IntOp::Add, &f64s, &f64s),
            Err(DataFusionError::Internal(_))
        ));
    }

    #[pg_test]
    fn plan_codec_resolves_each_udf_by_name() {
        for name in [INT_ADD_UDF_NAME, INT_SUB_UDF_NAME, INT_MUL_UDF_NAME] {
            let udf = crate::postgres::customscan::datafusion::udf_by_name(name)
                .unwrap_or_else(|| panic!("{name} is not registered"));
            assert_eq!(udf.name(), name);
        }
        assert!(crate::postgres::customscan::datafusion::udf_by_name("pdb_int_div").is_none());
    }
}
