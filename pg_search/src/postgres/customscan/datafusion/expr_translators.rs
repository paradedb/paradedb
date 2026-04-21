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

//! Extended PostgreSQL → DataFusion expression translation.
//!
//! This file augments `PredicateTranslator` (defined in `translator.rs`) with
//! additional node-type handlers via a split `impl` block. The methods here
//! cover expression shapes beyond the simple binary-operator / Var / Const
//! core — FuncExpr, NullTest, BooleanTest, CaseExpr, CoalesceExpr, NullIfExpr,
//! MinMaxExpr, ScalarArrayOpExpr, and CoerceViaIO — plus a UDF fallback hook
//! (`try_wrap_as_udf`) for opaque expressions.

use datafusion::common::ScalarValue;
use datafusion::logical_expr::expr::{Case, InList, ScalarFunction};
use datafusion::logical_expr::{col, lit, BinaryExpr, Expr, Operator, ScalarUDF};
use pgrx::{pg_sys, PgList};
use std::ffi::CStr;
use std::sync::Arc;

use crate::api::HashSet;
use crate::postgres::customscan::datafusion::translator::PredicateTranslator;
use crate::postgres::customscan::expr_eval::InputVarInfo;
use crate::postgres::customscan::pg_expr_udf::PgExprUdf;

const PVC_RECURSE_ALL: i32 = (pg_sys::PVC_RECURSE_AGGREGATES
    | pg_sys::PVC_RECURSE_WINDOWFUNCS
    | pg_sys::PVC_RECURSE_PLACEHOLDERS) as i32;

/// Short, `&'static str` label for the subset of PG node tags that reach
/// `try_wrap_as_udf`. Used only for building human-readable UDF names — a
/// rename in `pg_sys::NodeTag` won't silently shift the emitted label.
fn node_tag_label(tag: pg_sys::NodeTag) -> &'static str {
    match tag {
        pg_sys::NodeTag::T_OpExpr => "opexpr",
        pg_sys::NodeTag::T_FuncExpr => "funcexpr",
        pg_sys::NodeTag::T_BoolExpr => "boolexpr",
        pg_sys::NodeTag::T_NullTest => "nulltest",
        pg_sys::NodeTag::T_BooleanTest => "booleantest",
        pg_sys::NodeTag::T_CaseExpr => "caseexpr",
        pg_sys::NodeTag::T_CoalesceExpr => "coalesceexpr",
        pg_sys::NodeTag::T_NullIfExpr => "nullifexpr",
        pg_sys::NodeTag::T_MinMaxExpr => "minmaxexpr",
        pg_sys::NodeTag::T_ScalarArrayOpExpr => "scalararrayopexpr",
        pg_sys::NodeTag::T_CoerceViaIO => "coerceviaio",
        pg_sys::NodeTag::T_RelabelType => "relabeltype",
        _ => "expr",
    }
}

impl<'a> PredicateTranslator<'a> {
    /// Translate a `FuncExpr` node.
    ///
    /// Looks up the function's `(schema, name)` identity via the Postgres
    /// catalog, translates each argument recursively, and dispatches to
    /// [`Self::translate_known_func`]. Returns `None` when any argument
    /// fails to translate or the function isn't in the known-func map.
    pub(crate) unsafe fn translate_func_expr(&self, node: *mut pg_sys::Node) -> Option<Expr> {
        let func = node as *mut pg_sys::FuncExpr;
        let pg_args = PgList::<pg_sys::Node>::from_pg((*func).args);

        let df_args: Option<Vec<Expr>> =
            pg_args.iter_ptr().map(|arg| self.translate(arg)).collect();
        let df_args = df_args?;

        let funcid = (*func).funcid;
        let namespace_oid = pg_sys::get_func_namespace(funcid);
        let namespace_ptr = pg_sys::get_namespace_name(namespace_oid);
        let func_name_ptr = pg_sys::get_func_name(funcid);
        if namespace_ptr.is_null() || func_name_ptr.is_null() {
            return None;
        }

        let schema = CStr::from_ptr(namespace_ptr).to_str().ok()?;
        let name = CStr::from_ptr(func_name_ptr).to_str().ok()?;

        Self::translate_known_func(schema, name, df_args)
    }

    /// Map a `(schema, name, args)` triple to a native DataFusion `Expr`.
    ///
    /// Returns `None` for unrecognized functions — the caller can fall back
    /// to UDF wrapping.
    fn translate_known_func(schema: &str, name: &str, args: Vec<Expr>) -> Option<Expr> {
        // Scoped imports: these expr_fn modules contain very short names
        // (`abs`, `upper`, `round`, etc.) that could easily shadow or collide
        // if pulled in at file scope. Keep them local to this function.
        // Core items aren't glob-re-exported, so list them explicitly.
        use datafusion::functions::core::expr_fn::{coalesce, greatest, least, nullif};
        use datafusion::functions::math::expr_fn::*;
        use datafusion::functions::string::expr_fn::*;
        use datafusion::functions::unicode::expr_fn::*;

        let arity = args.len();
        let mut it = args.into_iter();
        match schema {
            "pg_catalog" => {
                match (name, arity) {
                    // string module
                    ("upper", 1) => Some(upper(it.next()?)),
                    ("lower", 1) => Some(lower(it.next()?)),
                    ("btrim" | "trim", _) => Some(btrim(it.collect())),
                    ("ltrim", _) => Some(ltrim(it.collect())),
                    ("rtrim", _) => Some(rtrim(it.collect())),
                    ("concat", _) => Some(concat(it.collect())),
                    ("ascii", 1) => Some(ascii(it.next()?)),
                    ("repeat", 2) => Some(repeat(it.next()?, it.next()?)),
                    ("starts_with", 2) => Some(starts_with(it.next()?, it.next()?)),
                    ("ends_with", 2) => Some(ends_with(it.next()?, it.next()?)),
                    ("replace", 3) => Some(replace(it.next()?, it.next()?, it.next()?)),

                    // unicode module
                    ("length" | "char_length" | "character_length", 1) => {
                        Some(character_length(it.next()?))
                    }
                    ("substr" | "substring", _) => match arity {
                        2 => Some(substr(it.next()?, it.next()?)),
                        3 => Some(substring(it.next()?, it.next()?, it.next()?)),
                        _ => None,
                    },
                    ("reverse", 1) => Some(reverse(it.next()?)),

                    // math module
                    ("abs", 1) => Some(abs(it.next()?)),
                    ("ceil" | "ceiling", 1) => Some(ceil(it.next()?)),
                    ("floor", 1) => Some(floor(it.next()?)),
                    ("round", _) => Some(round(it.collect())),
                    ("sqrt", 1) => Some(sqrt(it.next()?)),
                    ("power" | "pow", 2) => Some(power(it.next()?, it.next()?)),
                    ("sign", 1) => Some(signum(it.next()?)),
                    ("ln", 1) => Some(ln(it.next()?)),
                    ("log" | "log10", 1) => Some(log10(it.next()?)),

                    // core module
                    ("coalesce", _) => Some(coalesce(it.collect())),
                    ("nullif", 2) => Some(nullif(it.next()?, it.next()?)),
                    ("greatest", _) => Some(greatest(it.collect())),
                    ("least", _) => Some(least(it.collect())),

                    _ => None,
                }
            }

            _ => None,
        }
    }

    /// Translate a `NullTest` (x IS NULL / IS NOT NULL).
    pub(crate) unsafe fn translate_null_test(&self, node: *mut pg_sys::Node) -> Option<Expr> {
        let nt = node as *mut pg_sys::NullTest;
        let arg = self.translate((*nt).arg.cast())?;
        match (*nt).nulltesttype {
            pg_sys::NullTestType::IS_NULL => Some(Expr::IsNull(Box::new(arg))),
            pg_sys::NullTestType::IS_NOT_NULL => Some(Expr::IsNotNull(Box::new(arg))),
            _ => None,
        }
    }

    /// Translate a `BooleanTest` (x IS TRUE / IS NOT TRUE / …).
    pub(crate) unsafe fn translate_boolean_test(&self, node: *mut pg_sys::Node) -> Option<Expr> {
        let bt = node as *mut pg_sys::BooleanTest;
        let arg = self.translate((*bt).arg.cast())?;
        match (*bt).booltesttype {
            pg_sys::BoolTestType::IS_TRUE => Some(Expr::IsTrue(Box::new(arg))),
            pg_sys::BoolTestType::IS_NOT_TRUE => Some(Expr::IsNotTrue(Box::new(arg))),
            pg_sys::BoolTestType::IS_FALSE => Some(Expr::IsFalse(Box::new(arg))),
            pg_sys::BoolTestType::IS_NOT_FALSE => Some(Expr::IsNotFalse(Box::new(arg))),
            pg_sys::BoolTestType::IS_UNKNOWN => Some(Expr::IsUnknown(Box::new(arg))),
            pg_sys::BoolTestType::IS_NOT_UNKNOWN => Some(Expr::IsNotUnknown(Box::new(arg))),
            _ => None,
        }
    }

    /// Translate a `CaseExpr` (CASE [operand] WHEN … THEN … ELSE … END).
    pub(crate) unsafe fn translate_case_expr(&self, node: *mut pg_sys::Node) -> Option<Expr> {
        let case = node as *mut pg_sys::CaseExpr;

        let operand = if !(*case).arg.is_null() {
            Some(Box::new(self.translate((*case).arg.cast())?))
        } else {
            None
        };

        let when_list = PgList::<pg_sys::Node>::from_pg((*case).args);
        let mut when_then_expr: Vec<(Box<Expr>, Box<Expr>)> = Vec::with_capacity(when_list.len());
        for w in when_list.iter_ptr() {
            let cw = w as *mut pg_sys::CaseWhen;
            let when = self.translate((*cw).expr.cast())?;
            let then = self.translate((*cw).result.cast())?;
            when_then_expr.push((Box::new(when), Box::new(then)));
        }
        if when_then_expr.is_empty() {
            return None;
        }

        let else_expr = if !(*case).defresult.is_null() {
            Some(Box::new(self.translate((*case).defresult.cast())?))
        } else {
            None
        };

        Some(Expr::Case(Case {
            expr: operand,
            when_then_expr,
            else_expr,
        }))
    }

    /// Translate a `CoalesceExpr` to DataFusion's `coalesce(args)`.
    pub(crate) unsafe fn translate_coalesce_expr(&self, node: *mut pg_sys::Node) -> Option<Expr> {
        use datafusion::functions::core::expr_fn::coalesce;

        let ce = node as *mut pg_sys::CoalesceExpr;
        let pg_args = PgList::<pg_sys::Node>::from_pg((*ce).args);

        let df_args: Option<Vec<Expr>> =
            pg_args.iter_ptr().map(|arg| self.translate(arg)).collect();
        let df_args = df_args?;
        if df_args.is_empty() {
            return None;
        }

        Some(coalesce(df_args))
    }

    /// Translate a `NullIfExpr`. Postgres shares the `OpExpr` layout for this
    /// node, so we cast to `OpExpr` and read its two-argument list.
    pub(crate) unsafe fn translate_nullif_expr(&self, node: *mut pg_sys::Node) -> Option<Expr> {
        use datafusion::functions::core::expr_fn::nullif;

        debug_assert_eq!(
            (*node).type_,
            pg_sys::NodeTag::T_NullIfExpr,
            "translate_nullif_expr called with wrong NodeTag"
        );
        let op = node as *mut pg_sys::OpExpr;
        let args = PgList::<pg_sys::Node>::from_pg((*op).args);
        if args.len() != 2 {
            return None;
        }
        let left = self.translate(args.get_ptr(0)?)?;
        let right = self.translate(args.get_ptr(1)?)?;
        Some(nullif(left, right))
    }

    /// Translate a `MinMaxExpr` (GREATEST / LEAST).
    pub(crate) unsafe fn translate_min_max_expr(&self, node: *mut pg_sys::Node) -> Option<Expr> {
        use datafusion::functions::core::expr_fn::{greatest, least};

        let mmx = node as *mut pg_sys::MinMaxExpr;
        let pg_args = PgList::<pg_sys::Node>::from_pg((*mmx).args);

        let df_args: Option<Vec<Expr>> =
            pg_args.iter_ptr().map(|arg| self.translate(arg)).collect();
        let df_args = df_args?;
        if df_args.is_empty() {
            return None;
        }

        let is_greatest = matches!((*mmx).op, pg_sys::MinMaxOp::IS_GREATEST);
        Some(if is_greatest {
            greatest(df_args)
        } else {
            least(df_args)
        })
    }

    /// Translate a `ScalarArrayOpExpr` (`scalar = ANY(ARRAY[...])` /
    /// `scalar <> ALL(ARRAY[...])`) to `Expr::InList`.
    ///
    /// Only supports the ArrayExpr form of the RHS (literal array); other
    /// shapes (subqueries, runtime-constructed arrays) return `None`.
    pub(crate) unsafe fn translate_scalar_array_op_expr(
        &self,
        node: *mut pg_sys::Node,
    ) -> Option<Expr> {
        let saop = node as *mut pg_sys::ScalarArrayOpExpr;
        let args = PgList::<pg_sys::Node>::from_pg((*saop).args);
        if args.len() != 2 {
            return None;
        }

        let scalar = self.translate(args.get_ptr(0)?)?;

        let rhs = args.get_ptr(1)?;
        if (*rhs).type_ != pg_sys::NodeTag::T_ArrayExpr {
            return None;
        }
        let array_expr = rhs as *mut pg_sys::ArrayExpr;
        let elements = PgList::<pg_sys::Node>::from_pg((*array_expr).elements);
        let list: Option<Vec<Expr>> = elements.iter_ptr().map(|el| self.translate(el)).collect();
        let list = list?;

        let negated = !(*saop).useOr;

        Some(Expr::InList(InList {
            expr: Box::new(scalar),
            list,
            negated,
        }))
    }

    /// Translate a `CoerceViaIO`. Only transparent text-family coercions are
    /// accepted (TEXT ↔ VARCHAR ↔ NAME); everything else may change value
    /// semantics and is rejected.
    pub(crate) unsafe fn translate_coerce_via_io(&self, node: *mut pg_sys::Node) -> Option<Expr> {
        let coerce = node as *mut pg_sys::CoerceViaIO;
        let arg_node: *mut pg_sys::Node = (*coerce).arg.cast();
        if arg_node.is_null() {
            return None;
        }

        let source = pg_sys::exprType(arg_node);
        let target = (*coerce).resulttype;

        const TEXT_FAMILY: &[pg_sys::Oid] = &[pg_sys::TEXTOID, pg_sys::VARCHAROID, pg_sys::NAMEOID];
        let in_text = |oid: pg_sys::Oid| TEXT_FAMILY.contains(&oid);

        if in_text(source) && in_text(target) {
            self.translate(arg_node)
        } else {
            None
        }
    }

    /// Wrap an otherwise-untranslatable PostgreSQL expression subtree as a
    /// [`PgExprUdf`] scalar function. At execution time the UDF rehydrates the
    /// serialized node, populates a virtual tuple slot from its Arrow input
    /// columns, and delegates to `ExecEvalExpr`.
    ///
    /// Returns `None` when the expression can't be wrapped:
    ///   - result type is outside the supported set,
    ///   - some input Var can't be resolved against `self.sources` / mapper,
    ///   - `nodeToString` returns null.
    ///
    /// Called unconditionally as the final fallback in
    /// [`PredicateTranslator::translate`] — any subtree that fails native
    /// translation is wrapped here.
    pub(crate) unsafe fn try_wrap_as_udf(&self, node: *mut pg_sys::Node) -> Option<Expr> {
        let result_type_oid = pg_sys::exprType(node);
        PgExprUdf::get_result_type_to_arrow(result_type_oid)?;

        let var_list = pg_sys::pull_var_clause(node, PVC_RECURSE_ALL);
        let vars = PgList::<pg_sys::Var>::from_pg(var_list);

        let mut seen: HashSet<(pg_sys::Index, pg_sys::AttrNumber)> = HashSet::default();
        let mut input_exprs: Vec<Expr> = Vec::new();
        let mut input_vars: Vec<InputVarInfo> = Vec::new();

        for var_ptr in vars.iter_ptr() {
            if var_ptr.is_null() {
                continue;
            }
            let varno = (*var_ptr).varno as pg_sys::Index;
            let varattno = (*var_ptr).varattno;

            if varno == 0 || varattno <= 0 {
                continue;
            }
            // Skip internal join sentinels (INNER_VAR, OUTER_VAR) but allow
            // INDEX_VAR — it references the custom scan's own output tlist
            // and translate_var handles it via the mapper.
            if varno >= pg_sys::INNER_VAR as pg_sys::Index
                && varno != pg_sys::INDEX_VAR as pg_sys::Index
            {
                continue;
            }
            if !seen.insert((varno, varattno)) {
                continue;
            }

            let col_expr = self.translate_var(var_ptr)?;
            input_exprs.push(col_expr);
            input_vars.push(InputVarInfo {
                rti: varno,
                attno: varattno,
                type_oid: (*var_ptr).vartype,
                typmod: (*var_ptr).vartypmod,
                collation: (*var_ptr).varcollid,
            });
        }

        let node_str = pg_sys::nodeToString(node.cast());
        if node_str.is_null() {
            return None;
        }
        let pg_expr_string = CStr::from_ptr(node_str).to_string_lossy().into_owned();
        pg_sys::pfree(node_str.cast());

        let udf_name = PgExprUdf::stable_name(node_tag_label((*node).type_), &pg_expr_string);
        let udf = PgExprUdf::new(udf_name, pg_expr_string, input_vars, result_type_oid);

        Some(Expr::ScalarFunction(ScalarFunction::new_udf(
            Arc::new(ScalarUDF::new_from_impl(udf)),
            input_exprs,
        )))
    }

    // -----------------------------------------------------------------
    // Core node-type translators. These are the arms dispatched from
    // `translate()` in `translator.rs` — `pub(crate)` so the split impl
    // block in `translator.rs` can call them as methods on `&self`.
    // -----------------------------------------------------------------

    pub(crate) unsafe fn translate_op_expr(&self, op_expr: *mut pg_sys::OpExpr) -> Option<Expr> {
        let args = PgList::<pg_sys::Node>::from_pg((*op_expr).args);
        if args.len() != 2 {
            return None; // Only support binary operators for now
        }

        let left = self.translate(args.get_ptr(0)?)?;
        let right = self.translate(args.get_ptr(1)?)?;

        // Resolve the operator name straight from PG's catalog. The shared
        // `lookup_operator` table in `opexpr.rs` is scoped to the Tantivy
        // pushdown set and excludes arithmetic; DataFusion's translator has
        // its own set of native operators, so we go around it here.
        let opno = (*op_expr).opno;
        let op_name_ptr = pg_sys::get_opname(opno);
        if op_name_ptr.is_null() {
            return None;
        }
        let op_str = CStr::from_ptr(op_name_ptr).to_str().ok()?;
        let op = match op_str {
            "=" => Operator::Eq,
            "<>" | "!=" => Operator::NotEq,
            "<" => Operator::Lt,
            "<=" => Operator::LtEq,
            ">" => Operator::Gt,
            ">=" => Operator::GtEq,
            "+" => Operator::Plus,
            "-" => Operator::Minus,
            "*" => Operator::Multiply,
            "/" => Operator::Divide,
            "%" => Operator::Modulo,
            _ => return None,
        };

        Some(Expr::BinaryExpr(BinaryExpr::new(
            Box::new(left),
            op,
            Box::new(right),
        )))
    }

    pub(crate) unsafe fn translate_var(&self, var: *mut pg_sys::Var) -> Option<Expr> {
        let varno = (*var).varno as pg_sys::Index;
        let varattno = (*var).varattno;

        // INDEX_VAR is allowed because it represents a reference to the custom scan's output
        if varno != pg_sys::INDEX_VAR as pg_sys::Index
            && !self.sources.iter().any(|s| s.contains_rti(varno))
        {
            return None;
        }

        if let Some(ref mapper) = self.mapper {
            if let Some(expr) = mapper.map_var(varno, varattno) {
                return Some(expr);
            }
            return None;
        }

        Some(col("placeholder"))
    }

    pub(crate) unsafe fn translate_const(&self, c: *mut pg_sys::Const) -> Option<Expr> {
        if (*c).constisnull {
            return Some(lit(ScalarValue::Null));
        }

        let type_oid = (*c).consttype;
        let datum = (*c).constvalue;

        // Simple mapping for common types
        let scalar_value = match type_oid {
            pg_sys::INT2OID => {
                let val = datum.value() as i16;
                ScalarValue::Int16(Some(val))
            }
            pg_sys::INT4OID => {
                let val = datum.value() as i32;
                ScalarValue::Int32(Some(val))
            }
            pg_sys::INT8OID => {
                let val = datum.value() as i64;
                ScalarValue::Int64(Some(val))
            }
            pg_sys::FLOAT4OID => {
                let val = f32::from_bits(datum.value() as u32);
                ScalarValue::Float32(Some(val))
            }
            pg_sys::FLOAT8OID => {
                let val = f64::from_bits(datum.value() as u64);
                ScalarValue::Float64(Some(val))
            }
            pg_sys::BOOLOID => {
                let val = datum.value() != 0;
                ScalarValue::Boolean(Some(val))
            }
            _ => return None,
        };

        Some(lit(scalar_value))
    }

    pub(crate) unsafe fn translate_bool_expr(
        &self,
        bool_expr: *mut pg_sys::BoolExpr,
    ) -> Option<Expr> {
        let args = PgList::<pg_sys::Node>::from_pg((*bool_expr).args);

        match (*bool_expr).boolop {
            pg_sys::BoolExprType::AND_EXPR => {
                if args.len() < 2 {
                    return None;
                }
                let mut expr = self.translate(args.get_ptr(0)?)?;
                for i in 1..args.len() {
                    let right = self.translate(args.get_ptr(i)?)?;
                    expr = Expr::BinaryExpr(BinaryExpr::new(
                        Box::new(expr),
                        Operator::And,
                        Box::new(right),
                    ));
                }
                Some(expr)
            }
            pg_sys::BoolExprType::OR_EXPR => {
                if args.len() < 2 {
                    return None;
                }
                let mut expr = self.translate(args.get_ptr(0)?)?;
                for i in 1..args.len() {
                    let right = self.translate(args.get_ptr(i)?)?;
                    expr = Expr::BinaryExpr(BinaryExpr::new(
                        Box::new(expr),
                        Operator::Or,
                        Box::new(right),
                    ));
                }
                Some(expr)
            }
            pg_sys::BoolExprType::NOT_EXPR => {
                if args.len() != 1 {
                    return None;
                }
                let child = self.translate(args.get_ptr(0)?)?;
                Some(Expr::Not(Box::new(child)))
            }
            _ => None,
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use datafusion::logical_expr::col;
    use pgrx::prelude::*;
    use proptest::prelude::*;

    /// All `(name, arity)` pairs that `translate_known_func` must accept
    /// under `pg_catalog`. Keep in sync with the match arms in
    /// `PredicateTranslator::translate_known_func`.
    #[allow(dead_code)]
    const KNOWN_FUNCS: &[(&str, usize)] = &[
        // string module (fixed arity)
        ("upper", 1),
        ("lower", 1),
        ("ascii", 1),
        ("repeat", 2),
        ("starts_with", 2),
        ("ends_with", 2),
        ("replace", 3),
        // string module (variadic; always match)
        ("btrim", 0),
        ("btrim", 1),
        ("btrim", 2),
        ("trim", 0),
        ("trim", 1),
        ("trim", 2),
        ("ltrim", 0),
        ("ltrim", 1),
        ("ltrim", 2),
        ("rtrim", 0),
        ("rtrim", 1),
        ("rtrim", 2),
        ("concat", 0),
        ("concat", 1),
        ("concat", 2),
        ("concat", 3),
        // unicode module
        ("length", 1),
        ("char_length", 1),
        ("character_length", 1),
        ("substr", 2),
        ("substr", 3),
        ("substring", 2),
        ("substring", 3),
        ("reverse", 1),
        // math module
        ("abs", 1),
        ("ceil", 1),
        ("ceiling", 1),
        ("floor", 1),
        ("round", 0),
        ("round", 1),
        ("round", 2),
        ("sqrt", 1),
        ("power", 2),
        ("pow", 2),
        ("sign", 1),
        ("ln", 1),
        ("log", 1),
        ("log10", 1),
        // core module
        ("coalesce", 0),
        ("coalesce", 1),
        ("coalesce", 2),
        ("coalesce", 3),
        ("nullif", 2),
        ("greatest", 0),
        ("greatest", 1),
        ("greatest", 2),
        ("greatest", 3),
        ("least", 0),
        ("least", 1),
        ("least", 2),
        ("least", 3),
    ];

    #[allow(dead_code)]
    fn placeholder_args(arity: usize) -> Vec<Expr> {
        (0..arity).map(|i| col(format!("c{i}"))).collect()
    }

    // ---------- Test 1: translate_known_func coverage (pure Rust) ----------

    #[test]
    fn known_funcs_all_return_some() {
        for &(name, arity) in KNOWN_FUNCS {
            let args = placeholder_args(arity);
            let out = PredicateTranslator::translate_known_func("pg_catalog", name, args);
            assert!(
                out.is_some(),
                "expected Some for pg_catalog.{name}/{arity}, got None",
            );
        }
    }

    #[test]
    fn non_pg_catalog_schema_returns_none() {
        for &(name, arity) in KNOWN_FUNCS {
            for schema in ["public", "my_schema", "information_schema", ""] {
                let args = placeholder_args(arity);
                let out = PredicateTranslator::translate_known_func(schema, name, args);
                assert!(
                    out.is_none(),
                    "expected None for {schema}.{name}/{arity}, got Some",
                );
            }
        }
    }

    #[test]
    fn wrong_arity_returns_none() {
        // Fixed-arity functions rejected when called with the wrong count.
        let cases: &[(&str, usize)] = &[
            ("upper", 0),
            ("upper", 2),
            ("lower", 2),
            ("replace", 1),
            ("replace", 2),
            ("replace", 4),
            ("ascii", 0),
            ("ascii", 2),
            ("repeat", 1),
            ("repeat", 3),
            ("starts_with", 1),
            ("starts_with", 3),
            ("nullif", 1),
            ("nullif", 3),
            ("power", 1),
            ("power", 3),
            ("abs", 0),
            ("abs", 2),
            ("length", 0),
            ("length", 2),
            ("substr", 0),
            ("substr", 1),
            ("substr", 4),
        ];
        for &(name, arity) in cases {
            let args = placeholder_args(arity);
            let out = PredicateTranslator::translate_known_func("pg_catalog", name, args);
            assert!(
                out.is_none(),
                "expected None for pg_catalog.{name}/{arity}, got Some",
            );
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn unknown_func_name_returns_none(
            name in "[a-z_]{3,15}".prop_filter("not a known func", |n| {
                !KNOWN_FUNCS.iter().any(|(k, _)| *k == n)
            }),
            arity in 0usize..5,
        ) {
            let args = placeholder_args(arity);
            let out = PredicateTranslator::translate_known_func("pg_catalog", &name, args);
            prop_assert!(
                out.is_none(),
                "expected None for pg_catalog.{name}/{arity}, got Some",
            );
        }
    }

    // ---------- SQL-level tests (need live Postgres) ----------

    fn setup_test_tables() {
        pgrx::Spi::run(
            r#"
            DROP TABLE IF EXISTS prop_exclusions CASCADE;
            DROP TABLE IF EXISTS prop_items CASCADE;
            CREATE TABLE prop_items (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                value INT NOT NULL
            );
            CREATE TABLE prop_exclusions (
                id SERIAL PRIMARY KEY,
                pattern TEXT,
                threshold INT NOT NULL
            );
            INSERT INTO prop_items (name, value) VALUES
                ('alpha', 10), ('beta', 20), ('gamma', 30),
                ('', 0), ('UPPER', 100), ('  trimme  ', 50);
            INSERT INTO prop_exclusions (pattern, threshold) VALUES
                ('alpha', 5), (NULL, 10), ('', 15),
                ('BETA', 20), ('gamma', 25), ('  trimme  ', 30);
            CREATE INDEX prop_items_idx ON prop_items
                USING bm25 (id, name, value)
                WITH (
                    key_field='id',
                    text_fields='{"name": {"fast": true}}',
                    numeric_fields='{"value": {"fast": true}}'
                );
            CREATE INDEX prop_exclusions_idx ON prop_exclusions
                USING bm25 (id, pattern, threshold)
                WITH (
                    key_field='id',
                    text_fields='{"pattern": {"fast": true}}',
                    numeric_fields='{"threshold": {"fast": true}}'
                );
            "#,
        )
        .expect("setup_test_tables failed");
    }

    fn run_ids(query: &str) -> Vec<i32> {
        pgrx::Spi::connect(|client| {
            let args: [pgrx::datum::DatumWithOid; 0] = [];
            let result = client.select(query, None, &args)?;
            let mut ids = Vec::new();
            for row in result {
                if let Some(id) = row.get_by_name::<i32, _>("id")? {
                    ids.push(id);
                }
            }
            Ok::<_, pgrx::spi::Error>(ids)
        })
        .expect("run_ids failed")
    }

    fn explain_plan(query: &str) -> String {
        pgrx::Spi::connect(|client| {
            let args: [pgrx::datum::DatumWithOid; 0] = [];
            let result = client.select(query, None, &args)?;
            let mut out = String::new();
            for row in result {
                if let Some(line) = row.get::<String>(1)? {
                    out.push_str(&line);
                    out.push('\n');
                }
            }
            Ok::<_, pgrx::spi::Error>(out)
        })
        .expect("explain_plan failed")
    }

    fn arb_bool_expr() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("e.pattern IS NULL".to_string()),
            Just("e.pattern IS NOT NULL".to_string()),
            Just("length(e.pattern) > 3".to_string()),
            Just("upper(e.pattern) = upper(i.name)".to_string()),
            Just("COALESCE(e.pattern, '') = i.name".to_string()),
            // Arithmetic operators — native via translate_op_expr (+, -, *, /, %).
            Just("e.threshold + 10 > i.value".to_string()),
            Just("e.threshold * 2 > 50".to_string()),
            Just("i.value - e.threshold > 0".to_string()),
        ]
    }

    fn anti_join_query(expr: &str) -> String {
        // LIMIT is required for JoinScan to absorb a Semi/Anti subquery.
        format!(
            "SELECT i.id FROM prop_items i \
             WHERE NOT EXISTS ( \
                 SELECT 1 FROM prop_exclusions e \
                 WHERE e.id @@@ paradedb.all() AND ({expr}) \
             ) \
             AND i.id @@@ paradedb.all() \
             ORDER BY i.id LIMIT 10"
        )
    }

    // ---------- Test 2: JoinScan results match native PG ----------

    #[pg_test]
    fn joinscan_matches_native() {
        setup_test_tables();
        let cfg = ProptestConfig::with_cases(5);
        proptest!(cfg, |(expr in arb_bool_expr())| {
            let query = anti_join_query(&expr);

            pgrx::Spi::run("SET paradedb.enable_join_custom_scan = on").unwrap();
            let joinscan = run_ids(&query);

            pgrx::Spi::run("SET paradedb.enable_join_custom_scan = off").unwrap();
            let native = run_ids(&query);

            pgrx::Spi::run("SET paradedb.enable_join_custom_scan = on").unwrap();

            prop_assert_eq!(joinscan, native, "mismatch for expression: {}", expr);
        });
    }

    // ---------- Test 3: EXPLAIN shows JoinScan absorbed the expression ----------

    /// Only cross-table expressions — JoinScan's Semi/Anti absorption
    /// requires the predicate to reference Vars from both sides of the
    /// EXISTS/NOT EXISTS subquery. Single-table filters (e.g.
    /// `length(e.pattern) > 3`) get pushed down as heap filters on the
    /// inner scan and bypass JoinScan entirely.
    fn arb_absorbable_expr() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("upper(e.pattern) = i.name".to_string()),
            Just("lower(e.pattern) = lower(i.name)".to_string()),
            Just("abs(e.threshold) > i.value".to_string()),
            Just("COALESCE(e.pattern, '') = i.name".to_string()),
            Just("CASE WHEN e.pattern IS NOT NULL THEN e.pattern ELSE '' END = i.name".to_string(),),
            Just("e.pattern = i.name OR length(e.pattern) > 100".to_string()),
            Just("e.pattern = i.name OR e.pattern IS NULL".to_string()),
            Just("upper(e.pattern) = i.name OR e.threshold > i.value".to_string()),
            // Arithmetic cross-table predicates — now native OpExpr.
            Just("e.threshold * 2 > i.value".to_string()),
            Just("e.threshold + i.value > 100".to_string()),
        ]
    }

    #[pg_test]
    fn expression_is_absorbed_by_joinscan() {
        setup_test_tables();
        pgrx::Spi::run("SET paradedb.enable_join_custom_scan = on").unwrap();
        let cfg = ProptestConfig::with_cases(5);
        proptest!(cfg, |(expr in arb_absorbable_expr())| {
            let explain_query = format!("EXPLAIN (COSTS OFF) {}", anti_join_query(&expr));
            let plan = explain_plan(&explain_query);

            prop_assert!(
                plan.contains("ParadeDB Join Scan"),
                "expression '{}' was not absorbed by JoinScan. Plan:\n{}",
                expr,
                plan
            );
            prop_assert!(
                !plan.contains("JoinScan not used"),
                "expression '{}' triggered JoinScan decline. Plan:\n{}",
                expr,
                plan
            );
        });
    }

    // DISTINCT native-function coverage is already exercised
    // thoroughly by `tests/pg_regress/sql/join_distinct_expr.sql`, whose
    // expected output pins `upper(...)`, `character_length(...)`, `IS NULL`,
    // etc. in the physical plan. Repeating that via a proptest on the
    // minimal in-test schema triggers an unrelated JoinScan DISTINCT+PK-join
    // schema bug (column-name="" in apply_distinct_group_by), so the SQL
    // regression remains the authoritative coverage for that path.
}
