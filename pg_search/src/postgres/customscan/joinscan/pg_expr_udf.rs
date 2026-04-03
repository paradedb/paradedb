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
//! The PG expression state is lazily initialized on the first `invoke_batch` call
//! and reused for all subsequent calls. This follows the same pattern as
//! `HeapFieldFilter::initialized_expression` in `pg_search/src/query/heap_field_filter.rs`.

use std::any::Any;
use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::ptr::addr_of_mut;
use std::sync::Arc;

use datafusion::arrow::array::*;
use datafusion::arrow::datatypes::*;
use datafusion::common::{DataFusionError, Result};
use datafusion::logical_expr::{ColumnarValue, ScalarUDFImpl, Signature, Volatility};
use pgrx::pg_sys;
use pgrx::PgMemoryContexts;
use serde::{Deserialize, Serialize};

use crate::postgres::customscan::joinscan::build::InputVarInfo;

/// Prefix for PgExprUdf names. UDF names follow the pattern `{PREFIX}{index}`.
pub const PG_EXPR_UDF_PREFIX: &str = "pg_eval_expr_";

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
    /// Arrow return type (derived from result_type_oid)
    #[serde(skip, default = "PgExprUdf::default_return_type")]
    return_type: DataType,
    /// Pre-computed DataFusion Signature
    #[serde(skip, default = "PgExprUdf::default_signature")]
    signature: Signature,
    /// Lazily initialized PG expression evaluation state.
    #[serde(skip)]
    initialized_state: UnsafeCell<Option<PgExprState>>,
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

// SAFETY: PgExprUdf is only used within a single thread. DataFusion executes
// JoinScan plans single-threaded via target_partitions = 1. The UnsafeCell is
// never accessed concurrently.
unsafe impl Send for PgExprUdf {}
unsafe impl Sync for PgExprUdf {}

impl PgExprUdf {
    fn default_return_type() -> DataType {
        DataType::Utf8
    }

    fn default_signature() -> Signature {
        Signature::variadic_any(Volatility::Immutable)
    }

    /// Rebuild derived fields after deserialization.
    pub fn fixup_after_deserialize(&mut self) {
        self.return_type = pg_type_to_arrow_type(self.result_type_oid);
        let input_types: Vec<DataType> = self
            .input_vars
            .iter()
            .map(|v| pg_type_to_arrow_type(v.type_oid))
            .collect();
        self.signature = Signature::exact(input_types, Volatility::Immutable);
    }

    pub fn new(
        name: String,
        pg_expr_string: String,
        input_vars: Vec<InputVarInfo>,
        result_type_oid: pg_sys::Oid,
    ) -> Self {
        let return_type = pg_type_to_arrow_type(result_type_oid);

        let input_types: Vec<DataType> = input_vars
            .iter()
            .map(|v| pg_type_to_arrow_type(v.type_oid))
            .collect();
        let signature = Signature::exact(input_types, Volatility::Immutable);

        Self {
            name,
            pg_expr_string,
            input_vars,
            result_type_oid,
            return_type,
            signature,
            initialized_state: UnsafeCell::new(None),
        }
    }

    /// Lazily initialize PG expression state on first call.
    /// Follows the HeapFieldFilter pattern: allocate in TopTransactionContext
    /// so the ExprState/slot survive across DataFusion batch boundaries.
    ///
    /// # Safety
    /// Caller must ensure single-threaded access (guaranteed by target_partitions=1).
    unsafe fn get_or_init_state(&self) -> &PgExprState {
        // SAFETY: UnsafeCell access is safe because PgExprUdf is only used within
        // a single DataFusion partition (target_partitions=1).
        let state_ptr = self.initialized_state.get();
        if (*state_ptr).is_none() {
            let state = PgMemoryContexts::TopTransactionContext.switch_to(|_| {
                // Deserialize expression tree
                let c_str = std::ffi::CString::new(self.pg_expr_string.as_str())
                    .expect("pg_expr_string contains interior NUL byte");
                let expr_node =
                    pg_sys::stringToNode(c_str.as_ptr().cast_mut()) as *mut pg_sys::Expr;

                // Rewrite Var nodes to reference sequential slot positions (INNER_VAR)
                rewrite_var_nodes(expr_node.cast(), &self.input_vars);

                // Build TupleDesc for the synthetic input slot
                let tupdesc = build_tupdesc_for_inputs(&self.input_vars);

                // SAFETY: tupdesc is valid, TTSOpsVirtual is a static PG global.
                let slot = pg_sys::MakeSingleTupleTableSlot(tupdesc, &pg_sys::TTSOpsVirtual);

                // Create executor state
                let estate = pg_sys::CreateExecutorState();
                let econtext = pg_sys::CreateExprContext(estate);

                // Use ecxt_innertuple because Var nodes are rewritten to INNER_VAR.
                (*econtext).ecxt_innertuple = slot;

                // SAFETY: expr_node is a valid deserialized Node. Passing null PlanState
                // is safe — ExecInitExpr only uses it for Param resolution, which our
                // expressions don't contain.
                let expr_state = pg_sys::ExecInitExpr(expr_node, std::ptr::null_mut());

                PgExprState {
                    expr_state,
                    estate,
                    econtext,
                    slot,
                }
            });
            *state_ptr = Some(state);
        }
        (*state_ptr).as_ref().unwrap()
    }
}

impl Drop for PgExprUdf {
    fn drop(&mut self) {
        let state = self.initialized_state.get_mut();
        if let Some(s) = state.take() {
            // SAFETY: These PG free functions are safe to call during transaction
            // cleanup. The slot, econtext, and estate were allocated in
            // TopTransactionContext and are still valid at Drop time.
            unsafe {
                pg_sys::ExecDropSingleTupleTableSlot(s.slot);
                pg_sys::FreeExprContext(s.econtext, true);
                pg_sys::FreeExecutorState(s.estate);
            }
        }
    }
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
        let num_rows = args.number_rows;
        let arg_values = &args.args;

        // SAFETY: single-threaded access guaranteed by target_partitions=1.
        let pg_state = unsafe { self.get_or_init_state() };

        // TODO(#4604): Per-row ExecEvalExpr allocations for pass-by-reference types
        // (TEXT, etc.) accumulate for the entire batch in CurrentMemoryContext.
        // For large batches, consider wrapping the loop in a dedicated memory context
        // that is deleted after datums_to_arrow_array copies results to Arrow.
        // Current impact: bounded by batch_size (typically 8192) × avg datum size.
        let mut results = Vec::with_capacity(num_rows);
        let mut nulls = Vec::with_capacity(num_rows);

        for row_idx in 0..num_rows {
            unsafe {
                // SAFETY: slot was allocated in get_or_init_state and is valid.
                pg_sys::ExecClearTuple(pg_state.slot);

                // SAFETY: tts_values and tts_isnull are arrays of size >= natts.
                for (col_idx, arg) in arg_values.iter().enumerate() {
                    let (value, is_null) =
                        arrow_value_to_datum(arg, row_idx, self.input_vars[col_idx].type_oid)?;
                    (*pg_state.slot).tts_values.add(col_idx).write(value);
                    (*pg_state.slot).tts_isnull.add(col_idx).write(is_null);
                }
                (*pg_state.slot).tts_nvalid = arg_values.len() as i16;
                pg_sys::ExecStoreVirtualTuple(pg_state.slot);

                // SAFETY: expr_state and econtext are valid. ExecEvalExpr reads input
                // values from the slot via rewritten INNER_VAR Var nodes.
                //
                // Error handling follows the HeapFieldFilter pattern: direct unsafe
                // call under the #[pg_guard] boundary provided by exec_custom_scan().
                let mut is_null = false;
                let datum =
                    pg_sys::ExecEvalExpr(pg_state.expr_state, pg_state.econtext, &mut is_null);

                results.push(datum);
                nulls.push(is_null);
            }
        }

        let arrow_array =
            datums_to_arrow_array(&results, &nulls, self.result_type_oid, &self.return_type);

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
            std::ptr::null(), // no column name needed for virtual slot
            var_info.type_oid,
            var_info.typmod,
            0, // attdim
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
/// # Safety
/// Caller must ensure `row_idx` is in bounds for the array.
unsafe fn arrow_value_to_datum(
    col: &ColumnarValue,
    row_idx: usize,
    type_oid: pg_sys::Oid,
) -> Result<(pg_sys::Datum, bool)> {
    match col {
        ColumnarValue::Array(arr) => {
            if arr.is_null(row_idx) {
                Ok((pg_sys::Datum::from(0), true))
            } else {
                let datum = arrow_to_datum_single(arr.as_ref(), row_idx, type_oid)?;
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
                let datum = arrow_to_datum_single(arr.as_ref(), 0, type_oid)?;
                Ok((datum, false))
            }
        }
    }
}

/// Convert a single value from an Arrow array to a PG Datum.
///
/// # Safety
/// Caller must ensure `index` is in bounds and not null.
unsafe fn arrow_to_datum_single(
    array: &dyn Array,
    index: usize,
    type_oid: pg_sys::Oid,
) -> Result<pg_sys::Datum> {
    use pgrx::IntoDatum;

    match array.data_type() {
        DataType::Boolean => {
            let arr = array.as_any().downcast_ref::<BooleanArray>().unwrap();
            Ok(pg_sys::Datum::from(arr.value(index) as usize))
        }
        DataType::Int16 => {
            let arr = array.as_any().downcast_ref::<Int16Array>().unwrap();
            Ok(pg_sys::Datum::from(arr.value(index) as isize as usize))
        }
        DataType::Int32 => {
            let arr = array.as_any().downcast_ref::<Int32Array>().unwrap();
            Ok(pg_sys::Datum::from(arr.value(index) as isize as usize))
        }
        DataType::Int64 => {
            let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
            Ok(pg_sys::Datum::from(arr.value(index) as isize as usize))
        }
        DataType::UInt32 => {
            let arr = array.as_any().downcast_ref::<UInt32Array>().unwrap();
            Ok(pg_sys::Datum::from(arr.value(index) as usize))
        }
        DataType::UInt64 => {
            let arr = array.as_any().downcast_ref::<UInt64Array>().unwrap();
            Ok(pg_sys::Datum::from(arr.value(index) as usize))
        }
        DataType::Float32 => {
            let arr = array.as_any().downcast_ref::<Float32Array>().unwrap();
            Ok(pg_sys::Datum::from(arr.value(index).to_bits() as usize))
        }
        DataType::Float64 => {
            let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
            Ok(pg_sys::Datum::from(arr.value(index).to_bits() as usize))
        }
        DataType::Utf8 => {
            let arr = array.as_any().downcast_ref::<StringArray>().unwrap();
            let s = arr.value(index);
            Ok(s.into_datum().unwrap_or(pg_sys::Datum::from(0)))
        }
        DataType::Utf8View => {
            let arr = array.as_string_view();
            let s = arr.value(index);
            Ok(s.into_datum().unwrap_or(pg_sys::Datum::from(0)))
        }
        _ => {
            let pg_oid = pgrx::PgOid::from(type_oid);
            match crate::postgres::types_arrow::arrow_array_to_datum(array, index, pg_oid, None) {
                Ok(Some(datum)) => Ok(datum),
                Ok(None) => Err(DataFusionError::Internal(format!(
                    "arrow_array_to_datum returned None for type OID {}",
                    type_oid
                ))),
                Err(e) => Err(DataFusionError::Internal(format!(
                    "arrow_array_to_datum failed for type OID {}: {e}",
                    type_oid
                ))),
            }
        }
    }
}

/// Convert a Vec of result Datums back into an Arrow array.
fn datums_to_arrow_array(
    datums: &[pg_sys::Datum],
    nulls: &[bool],
    result_type_oid: pg_sys::Oid,
    _arrow_type: &DataType,
) -> Arc<dyn Array> {
    match result_type_oid {
        pg_sys::BOOLOID => {
            let mut builder = BooleanBuilder::with_capacity(datums.len());
            for (i, datum) in datums.iter().enumerate() {
                if nulls[i] {
                    builder.append_null();
                } else {
                    // SAFETY: BOOLOID Datum is a valid bool value (0 or 1).
                    builder.append_value(datum.value() != 0);
                }
            }
            Arc::new(builder.finish())
        }
        pg_sys::INT2OID => {
            let mut builder = Int16Builder::with_capacity(datums.len());
            for (i, datum) in datums.iter().enumerate() {
                if nulls[i] {
                    builder.append_null();
                } else {
                    builder.append_value(datum.value() as i16);
                }
            }
            Arc::new(builder.finish())
        }
        pg_sys::INT4OID => {
            let mut builder = Int32Builder::with_capacity(datums.len());
            for (i, datum) in datums.iter().enumerate() {
                if nulls[i] {
                    builder.append_null();
                } else {
                    builder.append_value(datum.value() as i32);
                }
            }
            Arc::new(builder.finish())
        }
        pg_sys::INT8OID => {
            let mut builder = Int64Builder::with_capacity(datums.len());
            for (i, datum) in datums.iter().enumerate() {
                if nulls[i] {
                    builder.append_null();
                } else {
                    builder.append_value(datum.value() as i64);
                }
            }
            Arc::new(builder.finish())
        }
        pg_sys::FLOAT4OID => {
            let mut builder = Float32Builder::with_capacity(datums.len());
            for (i, datum) in datums.iter().enumerate() {
                if nulls[i] {
                    builder.append_null();
                } else {
                    builder.append_value(f32::from_bits(datum.value() as u32));
                }
            }
            Arc::new(builder.finish())
        }
        pg_sys::FLOAT8OID => {
            let mut builder = Float64Builder::with_capacity(datums.len());
            for (i, datum) in datums.iter().enumerate() {
                if nulls[i] {
                    builder.append_null();
                } else {
                    builder.append_value(f64::from_bits(datum.value() as u64));
                }
            }
            Arc::new(builder.finish())
        }
        pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::NAMEOID => {
            let mut builder = StringBuilder::with_capacity(datums.len(), datums.len() * 32);
            for (i, datum) in datums.iter().enumerate() {
                if nulls[i] {
                    builder.append_null();
                } else {
                    // SAFETY: Pass-by-reference varlena: detoast, then read as &str
                    let text = unsafe {
                        let detoasted = pg_sys::pg_detoast_datum(datum.cast_mut_ptr());
                        let varlena = detoasted as *const pg_sys::varlena;
                        let len = pgrx::varlena::varsize_any_exhdr(varlena);
                        let data = pgrx::varlena::vardata_any(varlena);
                        std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                            data as *const u8,
                            len,
                        ))
                    };
                    builder.append_value(text);
                }
            }
            Arc::new(builder.finish())
        }
        _ => {
            // Fallback: convert to TEXT via PG's type output function, then store as Utf8.
            // SAFETY: result_type_oid is a valid type OID from planning time.
            let out_func_oid = unsafe {
                let mut typoutput = pg_sys::InvalidOid;
                let mut typisvarlena = false;
                pg_sys::getTypeOutputInfo(result_type_oid, &mut typoutput, &mut typisvarlena);
                typoutput
            };

            let mut builder = StringBuilder::with_capacity(datums.len(), datums.len() * 32);
            for (i, datum) in datums.iter().enumerate() {
                if nulls[i] {
                    builder.append_null();
                } else {
                    // SAFETY: out_func_oid is the output function for result_type_oid.
                    let text = unsafe {
                        let c_str = pg_sys::OidOutputFunctionCall(out_func_oid, *datum);
                        std::ffi::CStr::from_ptr(c_str)
                            .to_string_lossy()
                            .into_owned()
                    };
                    builder.append_value(&text);
                }
            }
            Arc::new(builder.finish())
        }
    }
}

/// Map a PostgreSQL type OID to an Arrow DataType.
fn pg_type_to_arrow_type(type_oid: pg_sys::Oid) -> DataType {
    match type_oid {
        pg_sys::BOOLOID => DataType::Boolean,
        pg_sys::INT2OID => DataType::Int16,
        pg_sys::INT4OID => DataType::Int32,
        pg_sys::INT8OID => DataType::Int64,
        pg_sys::FLOAT4OID => DataType::Float32,
        pg_sys::FLOAT8OID => DataType::Float64,
        pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::NAMEOID => DataType::Utf8,
        pg_sys::TIMESTAMPOID => DataType::Timestamp(TimeUnit::Microsecond, None),
        pg_sys::TIMESTAMPTZOID => DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
        pg_sys::DATEOID => DataType::Date32,
        pg_sys::NUMERICOID => DataType::Utf8, // Fallback to string representation
        _ => DataType::Utf8,                  // Default: convert to text representation
    }
}

// ---------------------------------------------------------------------------
// Var node rewriting
// ---------------------------------------------------------------------------

/// Context for the Var-rewriting walker.
struct VarRewriteCtx {
    /// Maps (original_varno, original_varattno) → 1-based sequential slot position.
    var_map: HashMap<(i32, pg_sys::AttrNumber), pg_sys::AttrNumber>,
}

/// Rewrite all Var nodes in an expression tree to reference sequential positions
/// in a synthetic tuple slot, based on the input_vars mapping.
///
/// Before: Var(varno=1, varattno=3) — references table 1, column 3
/// After:  Var(varno=INNER_VAR, varattno=1) — references slot position 1
///
/// We use INNER_VAR because ecxt_innertuple is set to our synthetic slot.
///
/// # Safety
/// `expr` must be a valid, mutable PG Node tree.
unsafe fn rewrite_var_nodes(expr: *mut pg_sys::Node, input_vars: &[InputVarInfo]) {
    use pgrx::pg_sys::expression_tree_walker;

    #[pgrx::pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if (*node).type_ == pg_sys::NodeTag::T_Var {
            let var = node as *mut pg_sys::Var;
            let ctx = &*(context as *const VarRewriteCtx);
            let key = ((*var).varno, (*var).varattno);
            if let Some(&new_attno) = ctx.var_map.get(&key) {
                (*var).varno = pg_sys::INNER_VAR;
                (*var).varattno = new_attno;
                (*var).varnosyn = pg_sys::INNER_VAR as pg_sys::Index;
                (*var).varattnosyn = new_attno;
            }
            return false; // Var is a leaf — no children to recurse into
        }

        // Non-Var node: recurse into children
        expression_tree_walker(node, Some(walker), context)
    }

    let mut ctx = VarRewriteCtx {
        var_map: input_vars
            .iter()
            .enumerate()
            .map(|(i, v)| ((v.rti as i32, v.attno), (i + 1) as pg_sys::AttrNumber))
            .collect(),
    };

    // Call walker directly on the root node (expression_tree_walker only visits children).
    walker(expr, addr_of_mut!(ctx).cast());
}
