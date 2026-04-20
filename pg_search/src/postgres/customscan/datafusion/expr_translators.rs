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

use datafusion::functions::core::expr_fn::{coalesce, greatest, least, nullif};
use datafusion::functions::math::expr_fn::{
    abs, ceil, floor, ln, log10, power, round, signum, sqrt,
};
use datafusion::functions::string::expr_fn::{
    ascii, btrim, concat, ends_with, lower, ltrim, repeat, replace, rtrim, starts_with, upper,
};
use datafusion::functions::unicode::expr_fn::{character_length, reverse, substr, substring};
use datafusion::logical_expr::expr::{Case, InList, ScalarFunction};
use datafusion::logical_expr::{Expr, ScalarUDF};
use pgrx::{pg_sys, PgList};
use std::ffi::CStr;
use std::sync::Arc;

use crate::api::HashSet;
use crate::postgres::customscan::datafusion::translator::PredicateTranslator;
use crate::postgres::customscan::expr_eval::InputVarInfo;
use crate::postgres::customscan::pg_expr_udf::{PgExprUdf, PG_EXPR_UDF_PREFIX};

const PVC_RECURSE_ALL: i32 = (pg_sys::PVC_RECURSE_AGGREGATES
    | pg_sys::PVC_RECURSE_WINDOWFUNCS
    | pg_sys::PVC_RECURSE_PLACEHOLDERS) as i32;

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
    /// Callers (see `translate()` in `translator.rs`) gate on
    /// `self.allow_udf_fallback` before invoking this method.
    pub(crate) unsafe fn try_wrap_as_udf(&self, node: *mut pg_sys::Node) -> Option<Expr> {
        let result_type_oid = pg_sys::exprType(node);
        PgExprUdf::try_result_type_to_arrow(result_type_oid)?;

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

        // Stable UDF names across runs
        let udf_name = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            pg_expr_string.hash(&mut hasher);
            let short_hash = format!("{:08x}", hasher.finish() as u32);

            let tag = format!("{:?}", (*node).type_)
                .strip_prefix("T_")
                .unwrap_or("expr")
                .to_lowercase();

            format!("{PG_EXPR_UDF_PREFIX}{tag}_{short_hash}")
        };
        let udf = PgExprUdf::new(udf_name, pg_expr_string, input_vars, result_type_oid);

        Some(Expr::ScalarFunction(ScalarFunction::new_udf(
            Arc::new(ScalarUDF::new_from_impl(udf)),
            input_exprs,
        )))
    }
}
