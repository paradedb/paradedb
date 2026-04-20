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

//! PgExprUdf — a DataFusion ScalarUDF wrapping PostgreSQL's ExecEvalExpr.
//!
//! This UDF takes one or more Arrow columns (the expression's input variables),
//! and for each row:
//! 1. Populates a Virtual tuple slot with the input values
//! 2. Calls ExecEvalExpr
//! 3. Collects the result into an output Arrow array
//!
//! The PG expression state is lazily initialized on the first invocation via
//! `OnceLock` and reused for all subsequent calls.
//!
//! All Datum↔Arrow conversion logic is delegated to
//! [`crate::postgres::types_arrow`] — this module is a consumer, not an
//! implementer of type conversions.

use std::any::Any;
use std::sync::OnceLock;

use datafusion::arrow::array::*;
use datafusion::arrow::datatypes::*;
use datafusion::common::{DataFusionError, Result};
use datafusion::logical_expr::{ColumnarValue, ScalarUDFImpl, Signature, Volatility};
use pgrx::pg_sys;
use pgrx::PgMemoryContexts;
use serde::{Deserialize, Serialize};

use super::expr_eval::{InputVarInfo, PreparedPgExpr};
use crate::postgres::types_arrow;

/// Prefix for PgExprUdf names. UDF names follow the pattern `{PREFIX}{index}`.
pub const PG_EXPR_UDF_PREFIX: &str = "pdb_eval_expr_";

/// A DataFusion ScalarUDF that wraps PostgreSQL's ExecEvalExpr.
#[derive(Serialize, Deserialize)]
pub struct PgExprUdf {
    name: String,
    /// Serialized PostgreSQL expression tree (from nodeToString)
    pg_expr_string: String,
    /// Input variable metadata with resolved type info from planning time
    input_vars: Vec<InputVarInfo>,
    /// PostgreSQL result type OID
    result_type_oid: pg_sys::Oid,
    /// Arrow return type for UDF OUTPUT (preserves PG expression result type)
    #[serde(skip, default = "PgExprUdf::default_return_type")]
    return_type: DataType,
    /// Pre-computed DataFusion Signature (input types match Tantivy widening)
    #[serde(skip, default = "PgExprUdf::default_signature")]
    signature: Signature,
    /// Lazily initialized PG expression evaluation state.
    #[serde(skip)]
    initialized_state: OnceLock<PgExprState>,
}

impl std::fmt::Debug for PgExprUdf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgExprUdf")
            .field("name", &self.name)
            .field("pg_expr_string", &self.pg_expr_string)
            .field("result_type_oid", &self.result_type_oid)
            .finish()
    }
}

impl PartialEq for PgExprUdf {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.pg_expr_string == other.pg_expr_string
            && self.result_type_oid == other.result_type_oid
    }
}

impl Eq for PgExprUdf {}

impl std::hash::Hash for PgExprUdf {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.pg_expr_string.hash(state);
        self.result_type_oid.hash(state);
    }
}

/// Cached PostgreSQL expression evaluation state.
/// Allocated in TopTransactionContext to survive across DataFusion batch boundaries.
struct PgExprState {
    expr_state: *mut pg_sys::ExprState,
    estate: *mut pg_sys::EState,
    econtext: *mut pg_sys::ExprContext,
    slot: *mut pg_sys::TupleTableSlot,
}

// SAFETY: PgExprState is only accessed within a single DataFusion partition
// (target_partitions=1). The raw pointers are not shared across threads.
unsafe impl Send for PgExprState {}
unsafe impl Sync for PgExprState {}

impl Drop for PgExprState {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        unsafe {
            pg_sys::ExecDropSingleTupleTableSlot(self.slot);
            pg_sys::FreeExprContext(self.econtext, true);
            pg_sys::FreeExecutorState(self.estate);
        }
    }
}

impl PgExprUdf {
    fn default_return_type() -> DataType {
        DataType::Utf8
    }

    fn default_signature() -> Signature {
        Signature::variadic_any(Volatility::Immutable)
    }

    /// Returns the Arrow `DataType` for a PG result type OID if `PgExprUdf`
    /// can evaluate it, or `None` if unsupported. This is the single gate for
    /// whether an expression can be UDF-wrapped.
    ///
    /// KEEP IN SYNC with the `eval_expr_to_arrow!` arms in
    /// [`Self::invoke_with_args`] — every OID accepted here must have a
    /// matching match arm there, and vice versa.
    pub fn try_result_type_to_arrow(oid: pg_sys::Oid) -> Option<DataType> {
        match oid {
            pg_sys::BOOLOID => Some(DataType::Boolean),
            pg_sys::INT2OID => Some(DataType::Int16),
            pg_sys::INT4OID => Some(DataType::Int32),
            pg_sys::INT8OID => Some(DataType::Int64),
            pg_sys::FLOAT4OID => Some(DataType::Float32),
            pg_sys::FLOAT8OID => Some(DataType::Float64),
            pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::NAMEOID => Some(DataType::Utf8),
            _ => None,
        }
    }

    /// Rebuild derived fields after deserialization.
    pub fn fixup_after_deserialize(&mut self) {
        self.return_type = types_arrow::pg_type_to_arrow(self.result_type_oid);
        let input_types: Vec<DataType> = self
            .input_vars
            .iter()
            .map(|v| types_arrow::pg_type_to_tantivy_arrow(v.type_oid))
            .collect();
        self.signature = Signature::exact(input_types, Volatility::Immutable);
    }

    pub fn new(
        name: String,
        pg_expr_string: String,
        input_vars: Vec<InputVarInfo>,
        result_type_oid: pg_sys::Oid,
    ) -> Self {
        let return_type = types_arrow::pg_type_to_arrow(result_type_oid);

        let input_types: Vec<DataType> = input_vars
            .iter()
            .map(|v| types_arrow::pg_type_to_tantivy_arrow(v.type_oid))
            .collect();
        let signature = Signature::exact(input_types, Volatility::Immutable);

        Self {
            name,
            pg_expr_string,
            input_vars,
            result_type_oid,
            return_type,
            signature,
            initialized_state: OnceLock::new(),
        }
    }

    /// Lazily initialize PG expression state on first call.
    unsafe fn get_or_init_state(&self) -> &PgExprState {
        self.initialized_state.get_or_init(|| {
            PgMemoryContexts::TopTransactionContext.switch_to(|_| {
                let prepared =
                    PreparedPgExpr::from_serialized(&self.pg_expr_string, &self.input_vars);

                let tupdesc = build_tupdesc_for_inputs(&self.input_vars);

                // SAFETY: tupdesc is valid, TTSOpsVirtual is a static PG global.
                let slot = pg_sys::MakeSingleTupleTableSlot(tupdesc, &pg_sys::TTSOpsVirtual);

                let estate = pg_sys::CreateExecutorState();
                let econtext = pg_sys::CreateExprContext(estate);

                // Use ecxt_innertuple because Var nodes are rewritten to INNER_VAR.
                (*econtext).ecxt_innertuple = slot;

                // SAFETY: expr_node is a valid deserialized+rewritten Node.
                let expr_state = pg_sys::ExecInitExpr(prepared.as_ptr(), std::ptr::null_mut());

                PgExprState {
                    expr_state,
                    estate,
                    econtext,
                    slot,
                }
            })
        })
    }
}

/// Populate a virtual tuple slot from Arrow input columns for one row.
///
/// # Safety
/// Caller must ensure `pg_state` pointers are valid and `row_idx` is in bounds.
unsafe fn populate_slot(
    pg_state: &PgExprState,
    args: &[ColumnarValue],
    input_vars: &[InputVarInfo],
    row_idx: usize,
) -> Result<()> {
    pg_sys::ExecClearTuple(pg_state.slot);
    for (col_idx, arg) in args.iter().enumerate() {
        let (val, null) = arrow_value_to_datum(arg, row_idx, input_vars[col_idx].type_oid)?;
        (*pg_state.slot).tts_values.add(col_idx).write(val);
        (*pg_state.slot).tts_isnull.add(col_idx).write(null);
    }
    (*pg_state.slot).tts_nvalid = args.len() as i16;
    pg_sys::ExecStoreVirtualTuple(pg_state.slot);
    Ok(())
}

/// Single-pass Datum→Arrow conversion: evaluate the PG expression for each row
/// and append the result directly to an Arrow builder. No intermediate Vec<Datum>.
///
/// Follows the `fetch_ff_column!` pattern from `fast_fields_helper.rs`.
///
/// KEEP IN SYNC with [`PgExprUdf::try_result_type_to_arrow`] — every PG type
/// accepted by that gate must have a matching arm here, and vice versa.
macro_rules! eval_expr_to_arrow {
    (
        $self_:expr, $num_rows:expr, $pg_state:expr, $args:expr, $input_vars:expr,
        $( $pg_type:pat => $convert:expr => $builder_init:expr ),* $(,)?
    ) => {
        match $self_.result_type_oid {
            $(
                $pg_type => {
                    let mut builder = $builder_init;
                    for row_idx in 0..$num_rows {
                        unsafe {
                            populate_slot($pg_state, $args, $input_vars, row_idx)?;
                            let mut is_null = false;
                            let datum = pg_sys::ExecEvalExpr(
                                $pg_state.expr_state,
                                $pg_state.econtext,
                                &mut is_null,
                            );
                            if is_null {
                                builder.append_null();
                            } else {
                                builder.append_value($convert(datum));
                            }
                        }
                    }
                    std::sync::Arc::new(builder.finish()) as std::sync::Arc<dyn Array>
                }
            )*
            other => {
                debug_assert!(
                    Self::try_result_type_to_arrow(other).is_none(),
                    "PgExprUdf::try_result_type_to_arrow accepts OID {other} but \
                     eval_expr_to_arrow! has no branch for it — keep the two in sync"
                );
                return Err(DataFusionError::Internal(format!(
                    "PgExprUdf: unsupported result type OID {other} — \
                     PgExprUdf::try_result_type_to_arrow must gate wrapping before this point"
                )));
            }
        }
    };
}

impl ScalarUDFImpl for PgExprUdf {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(self.return_type.clone())
    }

    fn invoke_with_args(
        &self,
        args: datafusion::logical_expr::ScalarFunctionArgs,
    ) -> Result<ColumnarValue> {
        // SAFETY: single-threaded access guaranteed by target_partitions=1.
        let pg_state = unsafe { self.get_or_init_state() };

        // A UDF over only constants has no input Vars — that's valid.
        // ExecEvalExpr evaluates it with an empty slot.

        let n = args.number_rows;
        let arg_values = &args.args;

        let arrow_array = eval_expr_to_arrow!(
            self, n, pg_state, arg_values, &self.input_vars,
            pg_sys::BOOLOID   => |d: pg_sys::Datum| d.value() != 0                    => BooleanBuilder::with_capacity(n),
            pg_sys::INT2OID   => |d: pg_sys::Datum| d.value() as i16                  => Int16Builder::with_capacity(n),
            pg_sys::INT4OID   => |d: pg_sys::Datum| d.value() as i32                  => Int32Builder::with_capacity(n),
            pg_sys::INT8OID   => |d: pg_sys::Datum| d.value() as isize as i64         => Int64Builder::with_capacity(n),
            pg_sys::FLOAT4OID => |d: pg_sys::Datum| f32::from_bits(d.value() as u32)  => Float32Builder::with_capacity(n),
            pg_sys::FLOAT8OID => |d: pg_sys::Datum| f64::from_bits(d.value() as u64)  => Float64Builder::with_capacity(n),
            pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::NAMEOID => |d: pg_sys::Datum| {
                use pgrx::FromDatum;
                String::from_datum(d, false).expect("non-null TEXT datum")
            } => StringBuilder::with_capacity(n, n * 32),
        );

        Ok(ColumnarValue::Array(arrow_array))
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Build a TupleDescriptor matching the input variables.
///
/// # Safety
/// Caller must ensure PG memory context is suitable for allocation.
unsafe fn build_tupdesc_for_inputs(input_vars: &[InputVarInfo]) -> *mut pg_sys::TupleDescData {
    let natts = input_vars.len();
    let tupdesc = pg_sys::CreateTemplateTupleDesc(natts as i32);

    for (i, var_info) in input_vars.iter().enumerate() {
        pg_sys::TupleDescInitEntry(
            tupdesc,
            (i + 1) as pg_sys::AttrNumber,
            std::ptr::null(),
            var_info.type_oid,
            var_info.typmod,
            0,
        );
        pg_sys::TupleDescInitEntryCollation(
            tupdesc,
            (i + 1) as pg_sys::AttrNumber,
            var_info.collation,
        );
    }

    tupdesc
}

/// Convert an Arrow ColumnarValue at a given row to a PostgreSQL (Datum, is_null) pair.
///
/// This is DataFusion-specific (wraps ColumnarValue). The actual Arrow→Datum
/// conversion is delegated to [`types_arrow::arrow_array_to_datum`].
///
/// # Safety
/// Caller must ensure `row_idx` is in bounds for the array.
unsafe fn arrow_value_to_datum(
    col: &ColumnarValue,
    row_idx: usize,
    type_oid: pg_sys::Oid,
) -> Result<(pg_sys::Datum, bool)> {
    let pg_oid = pgrx::PgOid::from(type_oid);
    match col {
        ColumnarValue::Array(arr) => {
            if arr.is_null(row_idx) {
                Ok((pg_sys::Datum::from(0), true))
            } else {
                let datum = types_arrow::arrow_array_to_datum(arr.as_ref(), row_idx, pg_oid, None)
                    .map_err(|e| {
                        DataFusionError::Internal(format!("arrow_array_to_datum failed: {e}"))
                    })?
                    .ok_or_else(|| {
                        DataFusionError::Internal(format!(
                            "arrow_array_to_datum returned None for type OID {}",
                            type_oid
                        ))
                    })?;
                Ok((datum, false))
            }
        }
        ColumnarValue::Scalar(scalar) => {
            if scalar.is_null() {
                Ok((pg_sys::Datum::from(0), true))
            } else {
                let arr = scalar.to_array().map_err(|e| {
                    DataFusionError::Internal(format!("ScalarValue to array failed: {e}"))
                })?;
                let datum = types_arrow::arrow_array_to_datum(arr.as_ref(), 0, pg_oid, None)
                    .map_err(|e| {
                        DataFusionError::Internal(format!("arrow_array_to_datum failed: {e}"))
                    })?
                    .ok_or_else(|| {
                        DataFusionError::Internal(format!(
                            "arrow_array_to_datum returned None for type OID {}",
                            type_oid
                        ))
                    })?;
                Ok((datum, false))
            }
        }
    }
}
