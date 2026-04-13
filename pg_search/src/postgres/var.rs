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

use crate::api::FieldName;
use crate::api::HashSet;
use crate::customscan::operator_oid;
use crate::nodecast;
use pgrx::pg_sys::NodeTag::{T_CoerceViaIO, T_Const, T_OpExpr, T_RelabelType, T_Var};
use pgrx::pg_sys::{expression_tree_walker, CoerceViaIO, Const, OpExpr, RelabelType, Var};
use pgrx::PgOid;
use pgrx::{is_a, pg_guard, pg_sys, FromDatum, PgList, PgRelation};
use std::ffi::CStr;
use std::ptr::addr_of_mut;
use std::sync::OnceLock;

/// Operator OIDs from pg_catalog, stable across PG14–PG18.
/// Source: `SELECT oid, oprcode FROM pg_operator WHERE …`
mod op_oids {
    use pgrx::pg_sys::Oid;
    pub const INT4PL: Oid = Oid::from_u32(551);
    pub const INT8PL: Oid = Oid::from_u32(684);
    pub const FLOAT4PL: Oid = Oid::from_u32(586);
    pub const FLOAT8PL: Oid = Oid::from_u32(591);
    pub const NUMERIC_ADD: Oid = Oid::from_u32(1758);
    pub const INT4MI: Oid = Oid::from_u32(555);
    pub const INT8MI: Oid = Oid::from_u32(688);
    pub const INT4MUL: Oid = Oid::from_u32(514);
    pub const INT8MUL: Oid = Oid::from_u32(686);
    pub const INT4DIV: Oid = Oid::from_u32(528);
}

/// Strip wrappers from `expr` that do not affect row ordering.
///
/// Returns a pointer to the first inner node whose shape the rest of
/// the planner recognizes (typically a `Var`). If no wrapper can be
/// safely removed, returns `expr` unchanged.
///
/// Safe to call with a null pointer; returns null in that case.
pub(crate) unsafe fn unwrap_order_preserving(mut expr: *mut pg_sys::Node) -> *mut pg_sys::Node {
    if expr.is_null() {
        return expr;
    }
    loop {
        match (*expr).type_ {
            pg_sys::NodeTag::T_RelabelType => {
                expr = (*(expr as *mut pg_sys::RelabelType)).arg as *mut pg_sys::Node;
            }
            pg_sys::NodeTag::T_CoerceToDomain => {
                expr = (*(expr as *mut pg_sys::CoerceToDomain)).arg as *mut pg_sys::Node;
            }
            pg_sys::NodeTag::T_OpExpr => {
                match try_unwrap_identity_opexpr(expr as *mut pg_sys::OpExpr) {
                    Some(inner) => expr = inner,
                    None => return expr,
                }
            }
            _ => return expr,
        }
    }
}

/// Attempt to unwrap an `OpExpr` of the shape `Var <op> Const` or `Const <op> Var`
/// where the operator is a known identity operation (e.g. `+ 0`, `* 1`).
unsafe fn try_unwrap_identity_opexpr(op: *mut pg_sys::OpExpr) -> Option<*mut pg_sys::Node> {
    // Must have exactly two args.
    let args = (*op).args;
    let args_list = PgList::<pg_sys::Node>::from_pg(args);
    if args_list.len() != 2 {
        return None;
    }
    let left = args_list.get_ptr(0)? as *mut pg_sys::Node;
    let right = args_list.get_ptr(1)? as *mut pg_sys::Node;

    // Match "Var-side <op> Const" or "Const <op> Var-side".
    // The "var side" is the non-Const operand — it may itself be a wrapped Var.
    let (var_side, const_side, const_on_right) = match ((*left).type_, (*right).type_) {
        (pg_sys::NodeTag::T_Const, _) => (right, left as *mut pg_sys::Const, false),
        (_, pg_sys::NodeTag::T_Const) => (left, right as *mut pg_sys::Const, true),
        _ => return None,
    };

    // Operator OID + identity-value check.
    if !is_identity_operation((*op).opno, const_side, const_on_right) {
        return None;
    }
    Some(var_side)
}

/// Check whether the given operator OID combined with the constant value
/// forms an identity operation that preserves row ordering.
///
/// `const_on_right` indicates whether the constant is the right operand.
/// For non-commutative operators like `-` and `/`, the constant must be on the right.
unsafe fn is_identity_operation(
    opno: pg_sys::Oid,
    konst: *mut pg_sys::Const,
    const_on_right: bool,
) -> bool {
    if konst.is_null() || (*konst).constisnull {
        return false;
    }

    match opno {
        // Addition: identity element is 0, commutative (either side)
        op_oids::INT4PL => i32::from_datum((*konst).constvalue, false) == Some(0),
        op_oids::INT8PL => i64::from_datum((*konst).constvalue, false) == Some(0),
        op_oids::FLOAT4PL => f32::from_datum((*konst).constvalue, false) == Some(0.0),
        op_oids::FLOAT8PL => f64::from_datum((*konst).constvalue, false) == Some(0.0),
        op_oids::NUMERIC_ADD => {
            // For numeric, extract as string and check for zero
            numeric_const_is_zero(konst)
        }

        // Subtraction: identity element is 0, but only when const is on the right
        // (0 - id inverts ordering)
        op_oids::INT4MI => const_on_right && i32::from_datum((*konst).constvalue, false) == Some(0),
        op_oids::INT8MI => const_on_right && i64::from_datum((*konst).constvalue, false) == Some(0),

        // Multiplication: identity element is 1, commutative (either side)
        op_oids::INT4MUL => i32::from_datum((*konst).constvalue, false) == Some(1),
        op_oids::INT8MUL => i64::from_datum((*konst).constvalue, false) == Some(1),

        // Division: identity element is 1, only when const is on the right
        op_oids::INT4DIV => {
            const_on_right && i32::from_datum((*konst).constvalue, false) == Some(1)
        }

        _ => false,
    }
}

/// Check if a numeric `Const` node represents zero.
unsafe fn numeric_const_is_zero(konst: *mut pg_sys::Const) -> bool {
    let numeric: Option<pgrx::AnyNumeric> = FromDatum::from_datum((*konst).constvalue, false);
    match numeric {
        Some(n) => n == pgrx::AnyNumeric::from(0i32),
        None => false,
    }
}

#[derive(Clone, Copy)]
pub enum VarContext {
    Planner(*mut pg_sys::PlannerInfo),
    Query(*mut pg_sys::Query),
    Exec(pg_sys::Oid),
}

/// Resolves an RTE_GROUP reference to its underlying Var.
///
/// In PostgreSQL 18+, GROUP BY creates a synthetic RTE_GROUP entry that wraps grouped columns.
/// This function extracts the underlying Var from the group expression list.
///
/// Returns `Some(var)` if:
/// - The varattno is valid (> 0)
/// - The groupexprs list is not null
/// - The expression at that position contains exactly one Var
///
/// Returns `None` otherwise, indicating the resolution should fall back to InvalidOid.
#[cfg(feature = "pg18")]
pub(crate) unsafe fn resolve_rte_group_var(
    rte: *mut pg_sys::RangeTblEntry,
    varattno: pg_sys::AttrNumber,
) -> Option<*mut pg_sys::Var> {
    // PG18: grouped columns are stored in rte->groupexprs, indexed by varattno.
    if varattno <= 0 || (*rte).groupexprs.is_null() {
        return None;
    }

    let group_exprs = PgList::<pg_sys::Node>::from_pg((*rte).groupexprs);
    let group_expr = group_exprs.get_ptr(varattno as usize - 1)?;
    // find_one_var returns None if the expression contains multiple Vars (e.g., "a + b"),
    // which is correct: we can only resolve single-column GROUP BY references here.
    let group_var = find_one_var(group_expr)?;

    Some(group_var)
}

impl VarContext {
    pub fn from_planner(root: *mut pg_sys::PlannerInfo) -> Self {
        Self::Planner(root)
    }

    pub fn from_query(parse: *mut pg_sys::Query) -> Self {
        Self::Query(parse)
    }

    pub fn from_exec(heaprelid: pg_sys::Oid) -> Self {
        Self::Exec(heaprelid)
    }

    pub fn var_relation(&self, var: *mut pg_sys::Var) -> (pg_sys::Oid, pg_sys::AttrNumber) {
        match self {
            Self::Planner(root) => {
                let (heaprelid, varattno, _) = unsafe { find_var_relation(var, *root) };
                (heaprelid, varattno)
            }
            Self::Query(parse) => unsafe {
                // Early return for null pointers
                if var.is_null() || parse.is_null() {
                    return (pg_sys::InvalidOid, (*var).varattno);
                }

                let query_ptr = *parse;
                let varno = (*var).varno;
                let rtable = (*query_ptr).rtable;

                // Early return for invalid rtable or varno
                if rtable.is_null() || varno <= 0 {
                    return (pg_sys::InvalidOid, (*var).varattno);
                }

                let rtable_list = PgList::<pg_sys::RangeTblEntry>::from_pg(rtable);
                let rte_index = (varno - 1) as usize;

                // Early return for out of bounds index
                if rte_index >= rtable_list.len() {
                    return (pg_sys::InvalidOid, (*var).varattno);
                }

                let varattno = (*var).varattno;
                let rte = match rtable_list.get_ptr(rte_index) {
                    Some(rte) => rte,
                    None => return (pg_sys::InvalidOid, varattno),
                };

                if (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION {
                    return ((*rte).relid, varattno);
                } else if (*rte).rtekind == pg_sys::RTEKind::RTE_SUBQUERY {
                    let subquery = (*rte).subquery;
                    if !subquery.is_null() {
                        let targetlist =
                            PgList::<pg_sys::TargetEntry>::from_pg((*subquery).targetList);
                        if varattno > 0 && (varattno as usize) <= targetlist.len() {
                            if let Some(te) = targetlist.get_ptr(varattno as usize - 1) {
                                if (*te).resorigtbl != pg_sys::InvalidOid {
                                    return ((*te).resorigtbl, (*te).resorigcol);
                                }
                            }
                        }
                    }
                }

                #[cfg(feature = "pg18")]
                if (*rte).rtekind == pg_sys::RTEKind::RTE_GROUP {
                    // PG18: grouped Vars point at RTE_GROUP, not the base relation.
                    if let Some(group_var) = resolve_rte_group_var(rte, varattno) {
                        let (heaprelid, group_attno) = self.var_relation(group_var);
                        if heaprelid != pg_sys::InvalidOid {
                            return (heaprelid, group_attno);
                        }
                    }
                }

                (pg_sys::InvalidOid, varattno)
            },
            Self::Exec(heaprelid) => (*heaprelid, unsafe { (*var).varattno }),
        }
    }
}

/// Given a [`pg_sys::Var`] and a [`pg_sys::PlannerInfo`], attempt to find the relation Oid that
/// contains the var.
///
/// It's possible the returned Oid will be [`pg_sys::Oid::INVALID`] if the Var doesn't eventually
/// come from a relation.
///
/// The returned [`pg_sys::AttrNumber`] is the physical attribute number in the relation the Var
/// is from.
pub unsafe fn find_var_relation(
    var: *mut pg_sys::Var,
    root: *mut pg_sys::PlannerInfo,
) -> (
    pg_sys::Oid,
    pg_sys::AttrNumber,
    Option<PgList<pg_sys::TargetEntry>>,
) {
    let query = (*root).parse;
    let varno = (*var).varno as pg_sys::Index;
    let rtable = (*query).rtable;
    let rtable_size = if !rtable.is_null() {
        pgrx::PgList::<pg_sys::RangeTblEntry>::from_pg(rtable).len()
    } else {
        0
    };

    // Bounds check: varno is 1-indexed, so it must be between 1 and rtable_size
    if varno == 0 || varno as usize > rtable_size {
        // This Var references an RTE that doesn't exist in the current context
        // This can happen with OR EXISTS subqueries where the Var comes from a subquery context
        // Return invalid values to signal this var cannot be processed
        return (pg_sys::InvalidOid, 0, None);
    }

    let rte = pg_sys::rt_fetch(varno, rtable);

    match (*rte).rtekind {
        // the Var comes from a relation
        pg_sys::RTEKind::RTE_RELATION => ((*rte).relid, (*var).varattno, None),

        // the Var comes from a subquery, so dig into its target list and find the original
        // table it comes from along with its original column AttributeNumber
        pg_sys::RTEKind::RTE_SUBQUERY => {
            if (*rte).subquery.is_null() {
                panic!("unable to determine Var relation as it belongs to a NULL subquery");
            }
            let targetlist = PgList::<pg_sys::TargetEntry>::from_pg((*(*rte).subquery).targetList);
            let te = targetlist
                .get_ptr((*var).varattno as usize - 1)
                .expect("var should exist in subquery TargetList");
            ((*te).resorigtbl, (*te).resorigcol, Some(targetlist))
        }

        // the Var comes from a CTE, so lookup that CTE and find it in the CTE's target list
        pg_sys::RTEKind::RTE_CTE => {
            let mut levelsup = (*rte).ctelevelsup;
            let mut cteroot = root;
            while levelsup > 0 {
                cteroot = (*cteroot).parent_root;
                if cteroot.is_null() {
                    // shouldn't happen
                    panic!(
                        "bad levelsup for CTE \"{}\"",
                        CStr::from_ptr((*rte).ctename).to_string_lossy()
                    )
                }
                levelsup -= 1;
            }

            let rte_ctename = CStr::from_ptr((*rte).ctename);
            let ctelist = PgList::<pg_sys::CommonTableExpr>::from_pg((*(*cteroot).parse).cteList);
            let mut matching_cte = None;
            for cte in ctelist.iter_ptr() {
                let ctename = CStr::from_ptr((*cte).ctename);

                if ctename == rte_ctename {
                    matching_cte = Some(cte);
                    break;
                }
            }

            let cte = matching_cte.unwrap_or_else(|| {
                panic!(
                    "unable to find cte named \"{}\"",
                    rte_ctename.to_string_lossy()
                )
            });

            if !is_a((*cte).ctequery, pg_sys::NodeTag::T_Query) {
                panic!("CTE is not a query")
            }
            let query = (*cte).ctequery.cast::<pg_sys::Query>();
            let targetlist = if !(*query).returningList.is_null() {
                PgList::<pg_sys::TargetEntry>::from_pg((*query).returningList)
            } else {
                PgList::<pg_sys::TargetEntry>::from_pg((*query).targetList)
            };
            let te = targetlist
                .get_ptr((*var).varattno as usize - 1)
                .expect("var should exist in cte TargetList");

            ((*te).resorigtbl, (*te).resorigcol, Some(targetlist))
        }

        // Custom scans involving named tuple stores (such as those created by `pg_ivm`) are not
        // supported.
        pg_sys::RTEKind::RTE_NAMEDTUPLESTORE => {
            (pg_sys::Oid::INVALID, pg_sys::InvalidAttrNumber as i16, None)
        }

        #[cfg(feature = "pg18")]
        pg_sys::RTEKind::RTE_GROUP => {
            // PG18: resolve grouped Vars back to their originating relation/column.
            if let Some(group_var) = resolve_rte_group_var(rte, (*var).varattno) {
                return find_var_relation(group_var, root);
            }
            (pg_sys::InvalidOid, pg_sys::InvalidAttrNumber as i16, None)
        }

        // Likewise, the safest bet for any other RTEKind that we do not recognize is to ignore it.
        rtekind => {
            pgrx::debug1!("Unsupported RTEKind in `find_var_relation`: {rtekind}");
            (pg_sys::Oid::INVALID, pg_sys::InvalidAttrNumber as i16, None)
        }
    }
}

/// Find all the Vars referenced in the specified node
pub unsafe fn find_vars(node: *mut pg_sys::Node) -> Vec<*mut pg_sys::Var> {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if let Some(var) = nodecast!(Var, T_Var, node) {
            let data = data.cast::<Data>();
            (*data).vars.push(var);
        }

        expression_tree_walker(node, Some(walker), data)
    }

    struct Data {
        vars: Vec<*mut pg_sys::Var>,
    }

    let mut data = Data { vars: Vec::new() };

    walker(node, addr_of_mut!(data).cast());
    data.vars
}

/// Given a [`pg_sys::Var`], attempt to find the [`FieldName`] that it references.
pub unsafe fn fieldname_from_var(
    heaprelid: pg_sys::Oid,
    var: *mut pg_sys::Var,
    varattno: pg_sys::AttrNumber,
) -> Option<FieldName> {
    if (*var).varattno == 0 || heaprelid == pg_sys::Oid::INVALID {
        return None;
    }
    // Check for InvalidOid before trying to open the relation
    if heaprelid == pg_sys::InvalidOid {
        return None;
    }
    let heaprel = PgRelation::open(heaprelid);
    let tupdesc = heaprel.tuple_desc();
    if varattno == pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber {
        Some("ctid".into())
    } else {
        tupdesc
            .get(varattno as usize - 1)
            .map(|attribute| attribute.name().into())
    }
}

/// Given a [`pg_sys::Node`], attempt to find the [`pg_sys::Var`] that it references.
///
/// If there is not exactly one Var in the node, then this function will return `None`.
pub unsafe fn find_one_var(node: *mut pg_sys::Node) -> Option<*mut pg_sys::Var> {
    let mut vars = find_vars(node);
    if vars.len() == 1 {
        Some(vars.pop().unwrap())
    } else {
        None
    }
}

/// Find an `Aggref` node in an expression tree using Postgres's `expression_tree_walker`
/// for robust traversal through all wrapper types (RelabelType, CoerceViaIO, FuncExpr, etc.).
pub unsafe fn find_one_aggref(node: *mut pg_sys::Node) -> Option<*mut pg_sys::Aggref> {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }
        if (*node).type_ == pg_sys::NodeTag::T_Aggref {
            (*(data as *mut Data)).found = node as *mut pg_sys::Aggref;
            return true;
        }
        expression_tree_walker(node, Some(walker), data)
    }

    struct Data {
        found: *mut pg_sys::Aggref,
    }

    let mut data = Data {
        found: std::ptr::null_mut(),
    };
    walker(node, addr_of_mut!(data).cast());
    if data.found.is_null() {
        None
    } else {
        Some(data.found)
    }
}

/// Given a [`pg_sys::Node`] and a [`pg_sys::PlannerInfo`], attempt to find the [`pg_sys::Var`] and
/// the [`FieldName`] that it references.
///
/// It is the caller's responsibility to ensure that the node contains a Var with a valid FieldName.
pub unsafe fn find_one_var_and_fieldname(
    context: VarContext,
    node: *mut pg_sys::Node,
) -> Option<(*mut pg_sys::Var, FieldName)> {
    if is_a(node, T_OpExpr) {
        let opexpr = node.cast::<OpExpr>();
        static JSON_OPERATOR_LOOKUP: OnceLock<HashSet<pg_sys::Oid>> = OnceLock::new();
        if JSON_OPERATOR_LOOKUP
            .get_or_init(|| initialize_json_operator_lookup())
            .contains(&(*opexpr).opno)
        {
            let var = find_one_var(node)?;
            let path = find_json_path(&context, node);
            return Some((var, path.join(".").into()));
        }
        None
    } else if is_a(node, pg_sys::NodeTag::T_SubscriptingRef) {
        // Handle PostgreSQL 14+ bracket notation: json['key']
        let var = find_one_var(node)?;
        let path = find_json_path(&context, node);
        Some((var, path.join(".").into()))
    } else if is_a(node, T_Var) {
        let var = node.cast::<Var>();
        let (heaprelid, varattno) = context.var_relation(var);
        Some((var, fieldname_from_var(heaprelid, var, varattno)?))
    } else if is_a(node, T_CoerceViaIO) {
        let expr = node.cast::<CoerceViaIO>();
        let arg = (*expr).arg;
        find_one_var_and_fieldname(context, arg as *mut pg_sys::Node)
    } else if is_a(node, T_RelabelType) {
        let relabel_type = node.cast::<RelabelType>();
        let arg = (*relabel_type).arg;
        find_one_var_and_fieldname(context, arg as *mut pg_sys::Node)
    } else {
        None
    }
}

/// Given a [`pg_sys::Node`] and a [`pg_sys::PlannerInfo`], attempt to find the JSON path that the
/// node references.
///
/// It is the caller's responsibility to ensure that the node is a JSON path expression.
#[inline(always)]
pub unsafe fn find_json_path(context: &VarContext, node: *mut pg_sys::Node) -> Vec<String> {
    let mut path = Vec::new();

    if is_a(node, T_Var) {
        let node = node as *mut Var;
        let (heaprelid, varattno) = context.var_relation(node);
        // Return empty path if we can't get a valid field name (e.g., due to out-of-bounds varno)
        if let Some(field_name) = fieldname_from_var(heaprelid, node, varattno) {
            path.push(field_name.root());
        }
        return path;
    } else if is_a(node, T_Const) {
        let node = node as *mut Const;
        if let PgOid::BuiltIn(oid) = PgOid::from((*node).consttype) {
            match oid {
                pg_sys::BuiltinOid::TEXTOID | pg_sys::BuiltinOid::VARCHAROID => {
                    if let Some(s) = String::from_datum((*node).constvalue, (*node).constisnull) {
                        path.push(s);
                    }
                }
                pg_sys::BuiltinOid::TEXTARRAYOID | pg_sys::BuiltinOid::VARCHARARRAYOID => {
                    if let Some(array) =
                        pgrx::Array::<String>::from_datum((*node).constvalue, (*node).constisnull)
                    {
                        path.extend(array.iter().flatten());
                    }
                }
                _ => {}
            }
        }

        return path;
    } else if is_a(node, T_OpExpr) {
        let node = node as *mut OpExpr;
        for expr in PgList::from_pg((*node).args).iter_ptr() {
            path.extend(find_json_path(context, expr));
        }
    } else if is_a(node, pg_sys::NodeTag::T_SubscriptingRef) {
        let node = node as *mut pg_sys::SubscriptingRef;
        // Extract container and subscript expressions for bracket notation
        path.extend(find_json_path(context, (*node).refexpr.cast()));
        for expr in PgList::from_pg((*node).refupperindexpr).iter_ptr() {
            path.extend(find_json_path(context, expr));
        }
    }

    path
}

#[inline(always)]
unsafe fn initialize_json_operator_lookup() -> HashSet<pg_sys::Oid> {
    const OPERATORS: [&str; 2] = ["->", "->>"];
    const TYPE_PAIRS: &[[&str; 2]] = &[
        ["json", "text"],
        ["jsonb", "text"],
        ["json", "int4"],
        ["jsonb", "int4"],
    ];

    let mut lookup = HashSet::default();
    for o in OPERATORS {
        for [l, r] in TYPE_PAIRS {
            lookup.insert(operator_oid(&format!("{o}({l},{r})")));
        }
    }

    lookup.insert(operator_oid("#>(json,text[])"));
    lookup.insert(operator_oid("#>(jsonb,text[])"));
    lookup.insert(operator_oid("#>>(json,text[])"));
    lookup.insert(operator_oid("#>>(jsonb,text[])"));

    lookup
}
