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

//! Vectorized float division with PostgreSQL semantics.
//!
//! Runs the checks of PostgreSQL's `float4_div` / `float8_div` per element and
//! raises the same errors. Both arguments must have the result type.

use std::convert::Infallible;
use std::sync::{Arc, LazyLock};

use arrow_array::cast::AsArray;
use arrow_array::types::{Float32Type, Float64Type};
use arrow_array::{Array, ArrayRef, Float32Array, Float64Array};
use datafusion::arrow::compute::try_binary;
use datafusion::arrow::datatypes::DataType;
use datafusion::arrow::error::ArrowError;
use datafusion::common::Result;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{
    ColumnarValue, ScalarFunctionArgs, ScalarUDF, ScalarUDFImpl, Signature, TypeSignature,
    Volatility,
};
use pgrx::{PgSqlErrorCode, pg_sys};

pub const FLOAT_DIV_UDF_NAME: &str = "pdb_float_div";

const OVERFLOW: &str = "value out of range: overflow";
const UNDERFLOW: &str = "value out of range: underflow";

macro_rules! pg_float_div {
    ($name:ident, $t:ty) => {
        /// PostgreSQL's `float4_div` / `float8_div`.
        fn $name(a: $t, b: $t) -> Result<$t, ArrowError> {
            if b == 0.0 && !a.is_nan() {
                return Err(ArrowError::DivideByZero);
            }
            let r = a / b;
            if r.is_infinite() && !a.is_infinite() {
                return Err(ArrowError::ArithmeticOverflow(OVERFLOW.into()));
            }
            if r == 0.0 && a != 0.0 && !b.is_infinite() {
                return Err(ArrowError::ArithmeticOverflow(UNDERFLOW.into()));
            }
            // Arrow sorts a negative NaN below every number, PostgreSQL sorts NaN
            // above them all.
            Ok(if r.is_nan() { <$t>::NAN } else { r })
        }
    };
}
pg_float_div!(float4_div, f32);
pg_float_div!(float8_div, f64);

fn divide(a: &ArrayRef, b: &ArrayRef) -> Result<ArrayRef, ArrowError> {
    match (a.data_type(), b.data_type()) {
        (DataType::Float32, DataType::Float32) => {
            let out: Float32Array = try_binary(
                a.as_primitive::<Float32Type>(),
                b.as_primitive::<Float32Type>(),
                float4_div,
            )?;
            Ok(Arc::new(out))
        }
        (DataType::Float64, DataType::Float64) => {
            let out: Float64Array = try_binary(
                a.as_primitive::<Float64Type>(),
                b.as_primitive::<Float64Type>(),
                float8_div,
            )?;
            Ok(Arc::new(out))
        }
        (l, r) => Err(ArrowError::InvalidArgumentError(format!(
            "{FLOAT_DIV_UDF_NAME} expects two Float32 or two Float64 arguments, got ({l}, {r})"
        ))),
    }
}

/// Raise PostgreSQL's error on the main thread. MPP workers get an execution
/// error instead.
fn raise(err: ArrowError) -> DataFusionError {
    let (code, msg) = match err {
        ArrowError::DivideByZero => (
            PgSqlErrorCode::ERRCODE_DIVISION_BY_ZERO,
            "division by zero".to_string(),
        ),
        ArrowError::ArithmeticOverflow(msg) => {
            (PgSqlErrorCode::ERRCODE_NUMERIC_VALUE_OUT_OF_RANGE, msg)
        }
        err => return err.into(),
    };
    let Err(e) =
        crate::postgres::customscan::mpp::try_run_on_main_thread(move |_| -> Result<Infallible> {
            pg_sys::ereport!(ERROR, code, msg);
        });
    e
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct FloatDivUdf {
    signature: Signature,
}

impl ScalarUDFImpl for FloatDivUdf {
    fn name(&self) -> &str {
        FLOAT_DIV_UDF_NAME
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
                "{FLOAT_DIV_UDF_NAME} expects two arguments, got {}",
                args.args.len()
            )));
        };
        let a = a.to_array_of_size(args.number_rows)?;
        let b = b.to_array_of_size(args.number_rows)?;
        divide(&a, &b).map(ColumnarValue::Array).map_err(raise)
    }
}

static FLOAT_DIV_UDF: LazyLock<Arc<ScalarUDF>> = LazyLock::new(|| {
    let exact = |t: DataType| TypeSignature::Exact(vec![t.clone(), t]);
    Arc::new(ScalarUDF::new_from_impl(FloatDivUdf {
        signature: Signature::one_of(
            vec![exact(DataType::Float32), exact(DataType::Float64)],
            Volatility::Immutable,
        ),
    }))
});

pub fn float_div_udf() -> Arc<ScalarUDF> {
    Arc::clone(&FLOAT_DIV_UDF)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overflowed<T>(r: Result<T, ArrowError>) -> bool {
        matches!(r, Err(ArrowError::ArithmeticOverflow(m)) if m == OVERFLOW)
    }

    fn underflowed<T>(r: Result<T, ArrowError>) -> bool {
        matches!(r, Err(ArrowError::ArithmeticOverflow(m)) if m == UNDERFLOW)
    }

    #[test]
    fn float8_matches_postgres() {
        let (inf, nan) = (f64::INFINITY, f64::NAN);
        assert_eq!(float8_div(1.0, 4.0).unwrap(), 0.25);
        assert_eq!(
            float8_div(-0.0, 2.0).unwrap().to_bits(),
            (-0.0f64).to_bits()
        );
        assert_eq!(float8_div(1.0, inf).unwrap(), 0.0);
        assert_eq!(
            float8_div(-1.0, inf).unwrap().to_bits(),
            (-0.0f64).to_bits()
        );
        assert_eq!(float8_div(inf, 2.0).unwrap(), inf);
        assert_eq!(float8_div(-inf, 2.0).unwrap(), -inf);
        assert!(float8_div(inf, inf).unwrap().is_nan());
        assert!(float8_div(nan, 0.0).unwrap().is_nan());
        assert!(float8_div(nan, 1.0).unwrap().is_nan());
        assert!(float8_div(1.0, nan).unwrap().is_nan());
        let canonical = float8_div(-nan, 1.0).unwrap();
        assert!(canonical.is_nan() && canonical.is_sign_positive());

        for (a, b) in [(1.0, 0.0), (1.0, -0.0), (0.0, 0.0), (inf, 0.0), (-1.0, 0.0)] {
            assert!(
                matches!(float8_div(a, b), Err(ArrowError::DivideByZero)),
                "{a} / {b}"
            );
        }
        assert!(overflowed(float8_div(f64::MAX, 0.5)));
        assert!(overflowed(float8_div(-f64::MAX, 0.5)));
        assert!(underflowed(float8_div(f64::MIN_POSITIVE, 1e300)));
    }

    #[test]
    fn float4_matches_postgres() {
        assert_eq!(float4_div(1.0, 3.0).unwrap(), 1.0f32 / 3.0);
        assert!(float4_div(f32::NAN, 0.0).unwrap().is_nan());
        assert!(matches!(
            float4_div(1.0, 0.0),
            Err(ArrowError::DivideByZero)
        ));
        // Overflows in float4 even though the quotient fits in float8.
        assert!(overflowed(float4_div(f32::MAX, 0.5)));
        assert!(underflowed(float4_div(f32::MIN_POSITIVE, 1e30)));
    }

    #[test]
    fn divide_skips_nulls_and_rejects_mixed_types() {
        let a: ArrayRef = Arc::new(Float64Array::from(vec![Some(6.0), None, Some(1.0)]));
        let b: ArrayRef = Arc::new(Float64Array::from(vec![Some(3.0), Some(0.0), None]));
        let out = divide(&a, &b).unwrap();
        assert_eq!(
            out.as_primitive::<Float64Type>(),
            &Float64Array::from(vec![Some(2.0), None, None])
        );

        let zero: ArrayRef = Arc::new(Float64Array::from(vec![0.0, 0.0, 0.0]));
        assert!(matches!(divide(&a, &zero), Err(ArrowError::DivideByZero)));

        let f4: ArrayRef = Arc::new(Float32Array::from(vec![1.0f32, 2.0, 3.0]));
        assert!(matches!(
            divide(&a, &f4),
            Err(ArrowError::InvalidArgumentError(_))
        ));
    }
}
