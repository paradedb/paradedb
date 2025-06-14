// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::api::operator::searchqueryinput_typoid;
use crate::api::{fieldname_typoid, FieldName, HashMap};
use crate::nodecast;
use crate::postgres::customscan::operator_oid;
use crate::postgres::customscan::pdbscan::qual_inspect::Qual;
use crate::postgres::var::{fieldname_from_var, find_var_relation};
use crate::schema::{SearchField, SearchIndexSchema};
use pgrx::{direct_function_call, pg_sys, IntoDatum, PgList};
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct PushdownField(FieldName);

impl PushdownField {
    /// Given a Postgres [`pg_sys::Var`] and a [`SearchIndexSchema`], try to create a [`PushdownField`].
    /// The purpose of this is to guard against the case where we mistakenly push down a field that's not indexed.
    ///
    /// Returns `Some(PushdownField)` if the field is found in the schema, `None` otherwise.
    /// If `None` is returned, a helpful warning is logged.
    pub unsafe fn try_new(
        root: *mut pg_sys::PlannerInfo,
        var: *mut pg_sys::Var,
        schema: &SearchIndexSchema,
    ) -> Option<Self> {
        let (heaprelid, varattno, _) = find_var_relation(var, root);
        if heaprelid == pg_sys::Oid::INVALID {
            return None;
        }
        let field = fieldname_from_var(heaprelid, var, varattno)?;
        schema.search_field(&field).map(|_| Self(field))
    }

    /// Create a new [`PushdownField`] from an attribute name.
    ///
    /// This does not verify if field can be pushed down and is intended to be used for testing.
    pub fn new(attname: &str) -> Self {
        Self(attname.into())
    }

    pub fn attname(&self) -> FieldName {
        self.0.clone()
    }

    pub fn search_field(&self, schema: &SearchIndexSchema) -> Option<SearchField> {
        schema.search_field(&self.0)
    }
}

macro_rules! pushdown {
    ($attname:expr, $opexpr:expr, $operator:expr, $rhs:ident) => {{
        let funcexpr = make_opexpr($attname, $opexpr, $operator, $rhs);

        if !is_complex(funcexpr.cast()) {
            Some(Qual::PushdownExpr { funcexpr })
        } else {
            Some(Qual::Expr {
                node: funcexpr.cast(),
                expr_state: std::ptr::null_mut(),
            })
        }
    }};
}

type PostgresOperatorOid = pg_sys::Oid;
type TantivyOperator = &'static str;

unsafe fn initialize_equality_operator_lookup() -> HashMap<PostgresOperatorOid, TantivyOperator> {
    const OPERATORS: [&str; 6] = ["=", ">", "<", ">=", "<=", "<>"];
    const TYPE_PAIRS: &[[&str; 2]] = &[
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
        // dates
        ["date", "date"],
        ["time", "time"],
        ["timetz", "timetz"],
        ["timestamp", "timestamp"],
        ["timestamptz", "timestamptz"],
        // text
        ["text", "text"],
        ["uuid", "uuid"],
    ];

    let mut lookup = HashMap::default();

    // tantivy doesn't support range operators on bools, so we can only support the equality operator
    lookup.insert(operator_oid("=(bool,bool)"), "=");

    for o in OPERATORS {
        for [l, r] in TYPE_PAIRS {
            lookup.insert(operator_oid(&format!("{o}({l},{r})")), o);
            if l != r {
                // types can be reversed too
                lookup.insert(operator_oid(&format!("{o}({r},{l})")), o);
            }
        }
    }

    lookup
}

/// Take a Postgres [`pg_sys::OpExpr`] pointer that is **not** of our `@@@` operator and try  to
/// convert it into one that is.
///
/// Returns `Some(Qual)` if we were able to convert it, `None` if not.
#[rustfmt::skip]
pub unsafe fn try_pushdown(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    opexpr: *mut pg_sys::OpExpr,
    schema: &SearchIndexSchema
) -> Option<Qual> {
    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
    let var = {
        // inspect the left-hand-side of the operator expression...
        let mut lhs = args.get_ptr(0)?;

        while (*lhs).type_ == pg_sys::NodeTag::T_RelabelType {
            // and keep following it as long as it's still a RelabelType
            let relabel_type = lhs as *mut pg_sys::RelabelType;
            lhs = (*relabel_type).arg as _;
        }
        nodecast!(Var, T_Var, lhs)?
    };
    let rhs = args.get_ptr(1)?;

    let pushdown = PushdownField::try_new(root, var, schema)?;
    let field = pushdown.search_field(schema)?;
    if field.is_text() && !field.is_keyword() {
        return None;
    }

    static EQUALITY_OPERATOR_LOOKUP: OnceLock<HashMap<pg_sys::Oid, &str>> = OnceLock::new();
    match EQUALITY_OPERATOR_LOOKUP.get_or_init(|| initialize_equality_operator_lookup()).get(&(*opexpr).opno) {
        Some(pgsearch_operator) => { 
            if let Some(pushed_down_qual) =  pushdown!(&pushdown.attname(), opexpr, pgsearch_operator, rhs) {
                // the `opexpr` is one we can pushdown
                if (*var).varno as pg_sys::Index == rti {
                    // and it's in this RTI, so we can use it directly
                    Some(pushed_down_qual)
                } else {
                    // it's not in this RTI, which means it's in some other table due to a join, so
                    // we need to indicate an arbitrary external var
                    Some(Qual::ExternalVar)
                }
            } else {
                None
            }
        },
        None => {
            // TODO:  support other types of OpExprs
            None
        }
    }
}

unsafe fn term_with_operator_procid() -> pg_sys::Oid {
    direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            // NB:  the SQL signature here needs to match our Rust implementation
            &[c"paradedb.term_with_operator(paradedb.fieldname, text, anyelement)".into_datum()],
        )
            .expect("the `paradedb.term_with_operator(paradedb.fieldname, text, anyelement)` function should exist")
}

unsafe fn make_opexpr(
    field: &FieldName,
    orig_opexor: *mut pg_sys::OpExpr,
    operator: &str,
    value: *mut pg_sys::Node,
) -> *mut pg_sys::FuncExpr {
    let paradedb_funcexpr: *mut pg_sys::FuncExpr =
        pg_sys::palloc0(size_of::<pg_sys::FuncExpr>()).cast();
    (*paradedb_funcexpr).xpr.type_ = pg_sys::NodeTag::T_FuncExpr;
    (*paradedb_funcexpr).funcid = term_with_operator_procid();
    (*paradedb_funcexpr).funcresulttype = searchqueryinput_typoid();
    (*paradedb_funcexpr).funcretset = false;
    (*paradedb_funcexpr).funcvariadic = false;
    (*paradedb_funcexpr).funcformat = pg_sys::CoercionForm::COERCE_EXPLICIT_CALL;
    (*paradedb_funcexpr).funccollid = pg_sys::InvalidOid;
    (*paradedb_funcexpr).inputcollid = (*orig_opexor).inputcollid;
    (*paradedb_funcexpr).location = (*orig_opexor).location;
    (*paradedb_funcexpr).args = {
        let fieldname = pg_sys::makeConst(
            fieldname_typoid(),
            -1,
            pg_sys::InvalidOid,
            -1,
            field.clone().into_datum().unwrap(),
            false,
            false,
        );
        let operator = pg_sys::makeConst(
            pg_sys::TEXTOID,
            -1,
            pg_sys::DEFAULT_COLLATION_OID,
            -1,
            operator.into_datum().unwrap(),
            false,
            false,
        );

        let mut args = PgList::<pg_sys::Node>::new();
        args.push(fieldname.cast());
        args.push(operator.cast());
        args.push(value.cast());

        args.into_pg()
    };

    paradedb_funcexpr
}

pub unsafe fn is_complex(root: *mut pg_sys::Node) -> bool {
    unsafe extern "C-unwind" fn walker(node: *mut pg_sys::Node, _: *mut core::ffi::c_void) -> bool {
        nodecast!(Var, T_Var, node).is_some()
            || nodecast!(Param, T_Param, node).is_some()
            || pg_sys::contain_volatile_functions(node)
            || pg_sys::expression_tree_walker(node, Some(walker), std::ptr::null_mut())
    }

    if root.is_null() {
        return false;
    }

    walker(root, std::ptr::null_mut())
}

/// Create an external filter expression for non-indexed predicates
/// This allows us to handle predicates that can't be pushed down to the index
/// but can still be evaluated via callback during Tantivy search
pub unsafe fn try_external_filter(
    root: *mut pg_sys::PlannerInfo,
    rti: pg_sys::Index,
    opexpr: *mut pg_sys::OpExpr,
) -> Option<Qual> {
    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);

    // Extract all variables from the expression to build the attribute mapping
    let mut attno_map = HashMap::default();
    let mut has_our_relation = false;

    // Check left-hand side
    if let Some(lhs_var) = extract_var_from_node(args.get_ptr(0)?) {
        if (*lhs_var).varno as pg_sys::Index == rti {
            has_our_relation = true;
            let (heaprelid, varattno, _) = find_var_relation(lhs_var, root);
            if heaprelid != pg_sys::Oid::INVALID {
                if let Some(field) = fieldname_from_var(heaprelid, lhs_var, varattno) {
                    attno_map.insert((*lhs_var).varattno, field);
                }
            }
        }
    }

    // Check right-hand side for variables (in case of var-to-var comparisons)
    if let Some(rhs_var) = extract_var_from_node(args.get_ptr(1)?) {
        if (*rhs_var).varno as pg_sys::Index == rti {
            has_our_relation = true;
            let (heaprelid, varattno, _) = find_var_relation(rhs_var, root);
            if heaprelid != pg_sys::Oid::INVALID {
                if let Some(field) = fieldname_from_var(heaprelid, rhs_var, varattno) {
                    attno_map.insert((*rhs_var).varattno, field);
                }
            }
        }
    }

    // Only create external filter if this expression involves our relation
    if has_our_relation && !attno_map.is_empty() {
        Some(Qual::FilterExpression {
            expr: opexpr.cast(),
            attno_map,
        })
    } else if !has_our_relation {
        // Expression involves other relations - treat as external
        Some(Qual::ExternalVar)
    } else {
        None
    }
}

/// Extract a Var node from a potentially complex expression
/// Handles RelabelType wrappers and other common expression patterns
unsafe fn extract_var_from_node(node: *mut pg_sys::Node) -> Option<*mut pg_sys::Var> {
    if node.is_null() {
        return None;
    }

    let mut current_node = node;

    // Follow RelabelType chains
    while (*current_node).type_ == pg_sys::NodeTag::T_RelabelType {
        let relabel_type = current_node as *mut pg_sys::RelabelType;
        current_node = (*relabel_type).arg as _;
    }

    // Check if we have a Var
    if (*current_node).type_ == pg_sys::NodeTag::T_Var {
        Some(current_node as *mut pg_sys::Var)
    } else {
        None
    }
}
