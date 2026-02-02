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

use crate::api::HashMap;
use crate::nodecast;
use crate::postgres::customscan::operator_oid;
use crate::postgres::types::ConstNode;
use pgrx::{pg_sys, PgList};
use std::sync::OnceLock;

pub type PostgresOperatorOid = pg_sys::Oid;
pub type TantivyOperator = &'static str;

pub trait TantivyOperatorExt {
    fn is_range(&self) -> bool;
    #[allow(unused)]
    fn is_eq(&self) -> bool;
    fn is_neq(&self) -> bool;
}

impl TantivyOperatorExt for TantivyOperator {
    fn is_range(&self) -> bool {
        *self == ">" || *self == ">=" || *self == "<" || *self == "<="
    }

    fn is_eq(&self) -> bool {
        *self == "="
    }

    fn is_neq(&self) -> bool {
        *self == "<>"
    }
}

pub const TEXT_TYPE_PAIRS: &[[&str; 2]] = &[["text", "text"], ["uuid", "uuid"]];
pub const NUMERIC_TYPE_PAIRS: &[[&str; 2]] = &[
    // integers
    ["int2", "int2"],
    ["int4", "int4"],
    ["int8", "int8"],
    ["int2", "int4"],
    ["int2", "int8"],
    ["int4", "int8"],
    // floats
    ["float4", "float4"],
    ["float8", "float8"],
    ["float4", "float8"],
    // numeric (DECIMAL) - PostgreSQL handles cross-type comparisons via implicit casting
    ["numeric", "numeric"],
    // dates
    ["date", "date"],
    ["time", "time"],
    ["timetz", "timetz"],
    ["timestamp", "timestamp"],
    ["timestamptz", "timestamptz"],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorAccepts {
    // text, uuid
    Text,
    // text, uuid, int, bool, etc.
    All,
}

pub unsafe fn initialize_equality_operator_lookup(
    accepts: OperatorAccepts,
) -> HashMap<PostgresOperatorOid, TantivyOperator> {
    const OPERATORS: [&str; 6] = ["=", ">", "<", ">=", "<=", "<>"];
    let mut lookup = HashMap::default();

    if accepts == OperatorAccepts::All {
        // tantivy doesn't support range operators on bools, so we can only support the equality operator
        lookup.insert(operator_oid("=(bool,bool)"), "=");
    }

    let type_pairs = match accepts {
        OperatorAccepts::Text => TEXT_TYPE_PAIRS,
        OperatorAccepts::All => &[NUMERIC_TYPE_PAIRS, TEXT_TYPE_PAIRS].concat(),
    };

    for o in OPERATORS {
        for [l, r] in type_pairs {
            lookup.insert(operator_oid(&format!("{o}({l},{r})")), o);
            if l != r {
                // types can be reversed too
                lookup.insert(operator_oid(&format!("{o}({r},{l})")), o);
            }
        }
    }

    lookup
}

#[derive(Debug)]
pub(crate) enum OpExpr {
    Array(*mut pg_sys::ScalarArrayOpExpr),
    Single(*mut pg_sys::OpExpr),
}

impl OpExpr {
    pub unsafe fn from_array(node: *mut pg_sys::Node) -> Option<Self> {
        nodecast!(ScalarArrayOpExpr, T_ScalarArrayOpExpr, node).map(OpExpr::Array)
    }

    pub unsafe fn from_single(node: *mut pg_sys::Node) -> Option<Self> {
        nodecast!(OpExpr, T_OpExpr, node).map(OpExpr::Single)
    }

    pub unsafe fn args(&self) -> PgList<pg_sys::Node> {
        match self {
            OpExpr::Array(expr) => PgList::<pg_sys::Node>::from_pg((*(*expr)).args),
            OpExpr::Single(expr) => PgList::<pg_sys::Node>::from_pg((*(*expr)).args),
        }
    }

    pub unsafe fn use_or(&self) -> Option<bool> {
        match self {
            OpExpr::Array(expr) => Some((*(*expr)).useOr),
            OpExpr::Single(_) => None,
        }
    }

    pub unsafe fn opno(&self) -> pg_sys::Oid {
        match self {
            OpExpr::Array(expr) => (*(*expr)).opno,
            OpExpr::Single(expr) => (*(*expr)).opno,
        }
    }

    pub unsafe fn inputcollid(&self) -> pg_sys::Oid {
        match self {
            OpExpr::Array(expr) => (*(*expr)).inputcollid,
            OpExpr::Single(expr) => (*(*expr)).inputcollid,
        }
    }

    pub unsafe fn location(&self) -> pg_sys::int32 {
        match self {
            OpExpr::Array(expr) => (*(*expr)).location,
            OpExpr::Single(expr) => (*(*expr)).location,
        }
    }

    pub fn is_text_binary(&self) -> bool {
        static TEXT_OPERATOR_LOOKUP: OnceLock<HashMap<PostgresOperatorOid, TantivyOperator>> =
            OnceLock::new();
        let opno = unsafe { self.opno() };

        TEXT_OPERATOR_LOOKUP
            .get_or_init(|| unsafe { initialize_equality_operator_lookup(OperatorAccepts::Text) })
            .get(&opno)
            .is_some()
    }
}

// ============================================================================
// Expression Unwrapping - Trait-based API
// ============================================================================

/// Trait for extracting PostgreSQL node types from expressions wrapped in type coercions.
///
/// Similar to `FromDatum`, this provides a type-safe way to extract specific node types
/// from PostgreSQL expression trees that may be wrapped in `RelabelType`, `CoerceViaIO`,
/// or `FuncExpr` coercion nodes.
///
/// # Example
/// ```ignore
/// // Extract a Var from a potentially wrapped expression
/// let var: Option<*mut pg_sys::Var> = UnwrapFromExpr::unwrap_from_expr(expr);
///
/// // Or with turbofish syntax
/// let var = <*mut pg_sys::Var>::unwrap_from_expr(expr);
/// ```
pub trait UnwrapFromExpr: Sized {
    /// Try to extract this node type from the expression, unwrapping all coercion layers
    /// including single-arg FuncExpr nodes (deep unwrapping).
    ///
    /// Use this when the expression might be wrapped in function-based type coercions
    /// (e.g., `float4` -> `float8` via a cast function).
    unsafe fn unwrap_from_expr(expr: *mut pg_sys::Expr) -> Option<Self>;

    /// Try to extract this node type from the expression, unwrapping only basic coercion
    /// layers (RelabelType, CoerceViaIO) without unwrapping FuncExpr nodes.
    ///
    /// Use this when you only want to strip simple type relabeling.
    unsafe fn unwrap_from_coercion(expr: *mut pg_sys::Expr) -> Option<Self>;
}

impl UnwrapFromExpr for *mut pg_sys::Var {
    unsafe fn unwrap_from_expr(expr: *mut pg_sys::Expr) -> Option<Self> {
        unwrap_expr(expr, |e| nodecast!(Var, T_Var, e))
    }

    unsafe fn unwrap_from_coercion(expr: *mut pg_sys::Expr) -> Option<Self> {
        unwrap_coercion(expr, |e| nodecast!(Var, T_Var, e))
    }
}

impl UnwrapFromExpr for *mut pg_sys::RowExpr {
    unsafe fn unwrap_from_expr(expr: *mut pg_sys::Expr) -> Option<Self> {
        unwrap_expr(expr, |e| nodecast!(RowExpr, T_RowExpr, e))
    }

    unsafe fn unwrap_from_coercion(expr: *mut pg_sys::Expr) -> Option<Self> {
        unwrap_coercion(expr, |e| nodecast!(RowExpr, T_RowExpr, e))
    }
}

impl UnwrapFromExpr for *mut pg_sys::Const {
    unsafe fn unwrap_from_expr(expr: *mut pg_sys::Expr) -> Option<Self> {
        unwrap_expr(expr, |e| nodecast!(Const, T_Const, e))
    }

    unsafe fn unwrap_from_coercion(expr: *mut pg_sys::Expr) -> Option<Self> {
        unwrap_coercion(expr, |e| nodecast!(Const, T_Const, e))
    }
}

impl UnwrapFromExpr for ConstNode {
    unsafe fn unwrap_from_expr(expr: *mut pg_sys::Expr) -> Option<Self> {
        unwrap_expr(expr, |e| ConstNode::try_from(e as *mut pg_sys::Node))
    }

    unsafe fn unwrap_from_coercion(expr: *mut pg_sys::Expr) -> Option<Self> {
        unwrap_coercion(expr, |e| ConstNode::try_from(e as *mut pg_sys::Node))
    }
}

// ============================================================================
// Expression Unwrapping - Internal helpers
// ============================================================================

/// Unwrap an expression from type coercion wrappers (RelabelType, CoerceViaIO, single-arg FuncExpr).
unsafe fn unwrap_expr<T, F>(mut expr: *mut pg_sys::Expr, mut extract: F) -> Option<T>
where
    F: FnMut(*mut pg_sys::Expr) -> Option<T>,
{
    loop {
        // Try to extract the target type at this level
        if let Some(result) = extract(expr) {
            return Some(result);
        }

        // Try to unwrap type coercion wrappers
        if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, expr) {
            expr = (*coerce).arg.cast();
            continue;
        }
        if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, expr) {
            expr = (*relabel).arg.cast();
            continue;
        }
        // Handle type coercion via single-arg function call (e.g., float4 -> float8)
        if let Some(func) = nodecast!(FuncExpr, T_FuncExpr, expr) {
            let args = PgList::<pg_sys::Node>::from_pg((*func).args);
            if args.len() == 1 {
                if let Some(arg) = args.get_ptr(0) {
                    expr = arg.cast();
                    continue;
                }
            }
        }

        // Can't unwrap further
        return None;
    }
}

/// Unwrap an expression from basic type coercion wrappers (RelabelType, CoerceViaIO only).
///
/// Unlike `unwrap_expr`, this does NOT unwrap single-arg FuncExpr nodes.
unsafe fn unwrap_coercion<T, F>(mut expr: *mut pg_sys::Expr, mut extract: F) -> Option<T>
where
    F: FnMut(*mut pg_sys::Expr) -> Option<T>,
{
    loop {
        if let Some(result) = extract(expr) {
            return Some(result);
        }
        if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, expr) {
            expr = (*coerce).arg.cast();
            continue;
        }
        if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, expr) {
            expr = (*relabel).arg.cast();
            continue;
        }
        return None;
    }
}

// ============================================================================
// Expression Comparison Utilities
// ============================================================================
//
// PostgreSQL can represent the same semantic expression in different forms:
// - Operators like `*` can appear as either OpExpr or FuncExpr
// - Type casts can wrap expressions in FuncExpr, RelabelType, or CoerceViaIO
// - The same column can have different varno values in different query contexts
//
// These utilities provide semantic comparison that handles these variations.
// ============================================================================

/// Compare two Vars for equality, ignoring varno and other context-dependent fields.
///
/// In CTE contexts, several fields may legitimately differ:
/// - varno: points to different range table entries (CTE vs original table)
/// - varnosyn, varattnosyn: syntactic variants that may differ in query rewriting
/// - varlevelsup: may differ in nested subquery contexts
/// - varnullingrels: nulling relationships may vary
/// - location: source location in the query text
///
/// The essential fields that must match for the Vars to represent the same column:
/// - varattno: the actual column number in the table
/// - vartype: the column's data type
/// - vartypmod: type modifier (e.g., varchar length)
/// - varcollid: collation (important for text comparisons)
pub unsafe fn vars_equal_ignoring_varno(a: *const pg_sys::Var, b: *const pg_sys::Var) -> bool {
    (*a).varattno == (*b).varattno
        && (*a).vartype == (*b).vartype
        && (*a).vartypmod == (*b).vartypmod
        && (*a).varcollid == (*b).varcollid
}

/// Get the implementation function OID for an OpExpr.
///
/// PostgreSQL may not populate `opfuncid` until planning time, so we fall back
/// to looking it up from `opno` if needed.
#[inline]
unsafe fn get_opexpr_funcid(opexpr: *const pg_sys::OpExpr) -> pg_sys::Oid {
    if (*opexpr).opfuncid != pg_sys::Oid::INVALID {
        (*opexpr).opfuncid
    } else {
        pg_sys::get_opcode((*opexpr).opno)
    }
}

/// Check if a function is a type cast function.
///
/// Type cast functions are identified by checking if a type with the same name
/// exists in the pg_type catalog. This is more robust than maintaining a
/// hardcoded list of cast functions.
///
/// Examples: `numeric()`, `int4()`, `text()`, etc.
unsafe fn is_type_cast_function(funcid: pg_sys::Oid) -> bool {
    if funcid == pg_sys::Oid::INVALID {
        return false;
    }

    let func_name = pg_sys::get_func_name(funcid);
    if func_name.is_null() {
        return false;
    }

    // A function is a type cast if there's a type with the same name
    pg_sys::TypenameGetTypid(func_name) != pg_sys::Oid::INVALID
}

/// Compare two PgList argument lists for semantic equality.
///
/// Returns true if both lists have the same length and all corresponding
/// arguments are semantically equal (using `expr_equal_ignoring_context`).
unsafe fn args_equal(args_a: &PgList<pg_sys::Node>, args_b: &PgList<pg_sys::Node>) -> bool {
    if args_a.len() != args_b.len() {
        return false;
    }

    for i in 0..args_a.len() {
        match (args_a.get_ptr(i), args_b.get_ptr(i)) {
            (Some(a), Some(b)) if expr_equal_ignoring_context(a, b) => continue,
            (None, None) => continue,
            _ => return false,
        }
    }

    true
}

/// Canonical arithmetic operations that can be represented differently across numeric types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArithmeticOp {
    Multiply,
    Divide,
    Add,
    Subtract,
}

/// Normalize arithmetic function names to their base operation.
///
/// PostgreSQL uses type-specific function names for arithmetic operations
/// (e.g., int4mul, float8mul, numeric_mul all implement multiplication).
/// This function maps them to a canonical operation for comparison.
fn normalize_arithmetic_op(name: &str) -> Option<ArithmeticOp> {
    // Pattern: {type}{op} where type is int2/int4/int8/float4/float8/numeric
    match name {
        "int2mul" | "int4mul" | "int8mul" | "float4mul" | "float8mul" | "numeric_mul" => {
            Some(ArithmeticOp::Multiply)
        }
        "int2div" | "int4div" | "int8div" | "float4div" | "float8div" | "numeric_div" => {
            Some(ArithmeticOp::Divide)
        }
        "int2pl" | "int4pl" | "int8pl" | "float4pl" | "float8pl" | "numeric_add" => {
            Some(ArithmeticOp::Add)
        }
        "int2mi" | "int4mi" | "int8mi" | "float4mi" | "float8mi" | "numeric_sub" => {
            Some(ArithmeticOp::Subtract)
        }
        _ => None,
    }
}

/// Check if two function OIDs implement semantically equivalent operations.
///
/// This handles cases where PostgreSQL uses different function implementations
/// for the same arithmetic operation on different numeric types (e.g., `int4pl`
/// and `float8pl` are both addition).
///
/// **Note**: This only compares the function identities, not their arguments.
/// Callers must separately verify argument equivalence if needed (see `args_equal`).
unsafe fn funcs_are_equivalent(funcid1: pg_sys::Oid, funcid2: pg_sys::Oid) -> bool {
    if funcid1 == funcid2 {
        return true;
    }
    if funcid1 == pg_sys::Oid::INVALID || funcid2 == pg_sys::Oid::INVALID {
        return false;
    }

    let name1 = pg_sys::get_func_name(funcid1);
    let name2 = pg_sys::get_func_name(funcid2);
    if name1.is_null() || name2.is_null() {
        return false;
    }

    let name1_str = std::ffi::CStr::from_ptr(name1).to_string_lossy();
    let name2_str = std::ffi::CStr::from_ptr(name2).to_string_lossy();

    // Check if both are arithmetic operations that normalize to the same op
    match (
        normalize_arithmetic_op(&name1_str),
        normalize_arithmetic_op(&name2_str),
    ) {
        (Some(op1), Some(op2)) => op1 == op2,
        _ => false,
    }
}

/// Check if an OpExpr matches a FuncExpr.
///
/// PostgreSQL represents operators like `*` either as OpExpr or FuncExpr.
/// They match if:
/// 1. The FuncExpr is a TYPE CAST wrapping an equivalent expression, OR
/// 2. The FuncExpr calls the same (or equivalent) function as the operator
///
/// **Important (Issue #3760)**: This does NOT match when the FuncExpr is a
/// different function that merely contains the OpExpr as an argument
/// (e.g., `abs(i - j)` should NOT match `i - j`).
unsafe fn opexpr_matches_funcexpr(
    opexpr: *const pg_sys::OpExpr,
    funcexpr: *const pg_sys::FuncExpr,
) -> bool {
    let func_funcid = (*funcexpr).funcid;

    // Case 1: FuncExpr is a type cast - look inside for the actual expression
    if is_type_cast_function(func_funcid) {
        let func_args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
        // Type casts have the expression as the first argument
        // (some like numeric() have a second typmod argument)
        return func_args
            .get_ptr(0)
            .is_some_and(|inner| expr_equal_ignoring_context(opexpr as *mut pg_sys::Node, inner));
    }

    // Case 2: FuncExpr must call the same (or equivalent) function
    let op_funcid = get_opexpr_funcid(opexpr);
    if !funcs_are_equivalent(op_funcid, func_funcid) {
        return false;
    }

    // Functions match - verify arguments are identical
    let func_args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
    let op_args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
    args_equal(&func_args, &op_args)
}

/// Compare two expressions for semantic equality.
///
/// This comparison ignores context-dependent fields that may differ between
/// index definition and query contexts:
/// - `varno`, `varlevelsup`: range table entry references
/// - `location`: source position in query text
/// - `varnosyn`, `varattnosyn`: syntactic variants from query rewriting
///
/// Use this instead of `pg_sys::equal` when comparing expressions from
/// different query contexts (e.g., index definition vs. WHERE clause).
pub unsafe fn expr_equal_ignoring_context(a: *mut pg_sys::Node, b: *mut pg_sys::Node) -> bool {
    if a.is_null() && b.is_null() {
        return true;
    }
    if a.is_null() || b.is_null() {
        return false;
    }

    let tag_a = (*a).type_;
    let tag_b = (*b).type_;

    // Handle OpExpr <-> FuncExpr cross-type comparison
    // PostgreSQL can represent operators either as OpExpr or FuncExpr
    match (tag_a, tag_b) {
        (pg_sys::NodeTag::T_OpExpr, pg_sys::NodeTag::T_FuncExpr) => {
            return opexpr_matches_funcexpr(
                a as *const pg_sys::OpExpr,
                b as *const pg_sys::FuncExpr,
            )
        }
        (pg_sys::NodeTag::T_FuncExpr, pg_sys::NodeTag::T_OpExpr) => {
            return opexpr_matches_funcexpr(
                b as *const pg_sys::OpExpr,
                a as *const pg_sys::FuncExpr,
            )
        }
        _ if tag_a != tag_b => return false,
        _ => {}
    }

    // Same node type - compare based on type
    match tag_a {
        pg_sys::NodeTag::T_Var => {
            vars_equal_ignoring_varno(a as *const pg_sys::Var, b as *const pg_sys::Var)
        }

        pg_sys::NodeTag::T_Const => {
            // Const nodes don't have context-dependent fields, use standard comparison
            pg_sys::equal(a.cast(), b.cast())
        }

        pg_sys::NodeTag::T_OpExpr => {
            let op_a = a as *const pg_sys::OpExpr;
            let op_b = b as *const pg_sys::OpExpr;

            (*op_a).opno == (*op_b).opno
                && (*op_a).opresulttype == (*op_b).opresulttype
                && (*op_a).opcollid == (*op_b).opcollid
                && (*op_a).inputcollid == (*op_b).inputcollid
                && args_equal(
                    &PgList::from_pg((*op_a).args),
                    &PgList::from_pg((*op_b).args),
                )
        }

        pg_sys::NodeTag::T_FuncExpr => {
            let func_a = a as *const pg_sys::FuncExpr;
            let func_b = b as *const pg_sys::FuncExpr;

            (*func_a).funcid == (*func_b).funcid
                && (*func_a).funcresulttype == (*func_b).funcresulttype
                && (*func_a).funcretset == (*func_b).funcretset
                && (*func_a).funccollid == (*func_b).funccollid
                && (*func_a).inputcollid == (*func_b).inputcollid
                && args_equal(
                    &PgList::from_pg((*func_a).args),
                    &PgList::from_pg((*func_b).args),
                )
        }

        pg_sys::NodeTag::T_RelabelType => {
            let relabel_a = a as *const pg_sys::RelabelType;
            let relabel_b = b as *const pg_sys::RelabelType;

            (*relabel_a).resulttype == (*relabel_b).resulttype
                && (*relabel_a).resulttypmod == (*relabel_b).resulttypmod
                && (*relabel_a).resultcollid == (*relabel_b).resultcollid
                && expr_equal_ignoring_context((*relabel_a).arg.cast(), (*relabel_b).arg.cast())
        }

        pg_sys::NodeTag::T_CoerceViaIO => {
            let coerce_a = a as *const pg_sys::CoerceViaIO;
            let coerce_b = b as *const pg_sys::CoerceViaIO;

            (*coerce_a).resulttype == (*coerce_b).resulttype
                && (*coerce_a).resultcollid == (*coerce_b).resultcollid
                && expr_equal_ignoring_context((*coerce_a).arg.cast(), (*coerce_b).arg.cast())
        }

        // Fall back to pg_sys::equal for other node types
        _ => pg_sys::equal(a.cast(), b.cast()),
    }
}

/// Check if an expression matches a node, unwrapping type coercions as needed.
///
/// This function handles the `pdb.alias` type specially - it will unwrap FuncExpr
/// nodes that cast to `pdb.alias`, but NOT other FuncExpr nodes (to avoid false
/// matches like `abs(i - j)` matching `i - j` - see Issue #3760).
pub unsafe fn expr_matches_node(
    node: *mut pg_sys::Node,
    indexed_expr: *mut pg_sys::Expr,
    type_is_alias: impl Fn(pg_sys::Oid) -> bool,
) -> bool {
    let mut reduced_expression = indexed_expr;
    loop {
        let inner_expression =
            if let Some(coerce) = nodecast!(CoerceViaIO, T_CoerceViaIO, reduced_expression) {
                (*coerce).arg
            } else {
                reduced_expression
            };

        // Use our custom comparison that ignores context-dependent fields
        if expr_equal_ignoring_context(node, inner_expression.cast()) {
            return true;
        }

        if let Some(relabel) = nodecast!(RelabelType, T_RelabelType, reduced_expression) {
            reduced_expression = (*relabel).arg.cast();
            continue;
        }

        // a cast to `pdb.alias` can make it a `FuncExpr` that we need to unwrap
        // Only unwrap pdb.alias casts; unwrapping other FuncExprs like abs() causes false index matches (#3760).
        if let Some(func) = nodecast!(FuncExpr, T_FuncExpr, reduced_expression) {
            if type_is_alias((*func).funcresulttype) {
                let args = PgList::<pg_sys::Node>::from_pg((*func).args);
                if args.len() == 1 {
                    if let Some(arg) = args.get_ptr(0) {
                        reduced_expression = arg.cast();
                        continue;
                    }
                }
            }
        }

        return false;
    }
}
