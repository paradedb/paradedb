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

//! Shared projection utilities for custom scans.
//!
//! This module contains functions for handling wrapped expressions where
//! placeholder functions are nested inside other functions
//! (e.g., `jsonb_pretty(pdb.agg(...))`).
#![allow(clippy::unnecessary_cast)]

use crate::api::FieldName;
use crate::api::HashMap;
use crate::api::Varno;
use crate::api::agg_fn_oid;
use crate::api::operator::ReturnedNodePointer;
use crate::nodecast;
use crate::postgres::customscan::basescan::projections::snippet::{
    SnippetType, extract_snippet, extract_snippet_positions, extract_snippets,
};
use crate::postgres::customscan::range_table::{rte_is_parent, rte_is_partitioned};
use crate::postgres::node::{NodeExt, WalkControl};
use crate::postgres::var::{VarContext, find_one_var_and_fieldname};
use pgrx::{Internal, IntoDatum, PgList, direct_function_call, pg_extern, pg_guard, pg_sys};
use std::ptr::addr_of_mut;
use tantivy::snippet::SnippetGenerator;

/// Get the Oid of a placeholder function to use in the target list of aggregate custom scans.
pub(crate) fn placeholder_procid() -> pg_sys::Oid {
    unsafe {
        let agg_fn_oid = agg_fn_oid();
        if agg_fn_oid != pg_sys::InvalidOid {
            agg_fn_oid
        } else {
            // Fallback to now() if pdb.agg_fn doesn't exist yet (e.g., during extension creation)
            direct_function_call::<pg_sys::Oid>(pg_sys::regprocedurein, &[c"now()".into_datum()])
                .expect("the `now()` function should exist")
        }
    }
}

/// Create a placeholder target list for aggregate custom scans.
///
/// This is called AFTER `replace_aggrefs_in_target_list` has replaced Aggrefs with FuncExprs.
/// It performs three main tasks:
/// 1. Replaces `pdb.agg_fn` FuncExprs with placeholder Vars that read the aggregate values of
///    the current row from the slot of a [`PlaceholderProjection`].
/// 2. Replaces grouping column expressions with `INDEX_VAR` nodes. Because `AggregateScan`
///    groups by columns directly in Tantivy, it yields a virtual scan slot with only the
///    grouping column and aggregate results (unlike `BaseScan` which yields the heap relation).
///    By converting the original base relation Vars into `INDEX_VAR`s, `ExecProject` knows to
///    fetch the value from the virtual slot's attributes instead of attempting to evaluate
///    the original expressions (which would otherwise fail due to missing base columns).
/// 3. In mixed expressions like `COUNT(*)::text || category`, replaces each sub-expression
///    that equals a whole grouping expression with an `INDEX_VAR` at that grouping column's
///    slot attribute, so `ExecProject` reads the value from the virtual slot. Only whole
///    expressions are matched: for `GROUP BY reverse(category)`, the slot already holds
///    `reverse(category)`, so the `Var` inside it must not be rewritten on its own.
///
/// Returns: (placeholder_targetlist, placeholders, needs_projection)
/// - placeholder_targetlist: target list with FuncExprs replaced by placeholder Vars and grouping columns converted to `INDEX_VAR`s.
/// - placeholders: the column of `columns` and its type for each placeholder, indexed by target entry position.
/// - needs_projection: true if projection is needed (e.g., wrapped expressions exist).
pub(crate) fn create_placeholder_targetlist(
    targetlist: *mut pg_sys::List,
    columns: &mut PlaceholderColumns,
) -> (
    *mut pg_sys::List,
    Vec<Option<(PlaceholderColumn, pg_sys::Oid)>>,
    bool,
) {
    if targetlist.is_null() {
        return (std::ptr::null_mut(), Default::default(), false);
    }

    let placeholder_funcid = placeholder_procid();
    let targetlist_pg = unsafe { PgList::<pg_sys::TargetEntry>::from_pg(targetlist) };

    // Check if any target entries have wrapped placeholder FuncExprs (not top-level)
    let needs_projection = targetlist_pg.iter_ptr().any(|te| {
        if te.is_null() || unsafe { (*te).expr.is_null() } {
            return false;
        }
        let te = unsafe { &*te };
        // Check if the expression is NOT a direct FuncExpr placeholder but CONTAINS one
        let is_top_level_placeholder = unsafe { (*te.expr).type_ } == pg_sys::NodeTag::T_FuncExpr
            && unsafe { (*(te.expr as *mut pg_sys::FuncExpr)).funcid } == placeholder_funcid;

        !is_top_level_placeholder && te.expr.contains_functions(&[placeholder_funcid])
    });

    if !needs_projection {
        return (std::ptr::null_mut(), Default::default(), false);
    }

    // Grouping expressions and their slot attrs, used to rewrite them inside mixed expressions.
    let mut grouping_slots: Vec<(*mut pg_sys::Node, pg_sys::AttrNumber)> = Vec::new();
    for (i, te) in targetlist_pg.iter_ptr().enumerate() {
        if te.is_null() || unsafe { (*te).expr.is_null() } {
            continue;
        }
        let te = unsafe { &*te };
        let is_top_level_placeholder = unsafe { (*te.expr).type_ } == pg_sys::NodeTag::T_FuncExpr
            && unsafe { (*(te.expr as *mut pg_sys::FuncExpr)).funcid } == placeholder_funcid;
        let contains_placeholder =
            is_top_level_placeholder || te.expr.contains_functions(&[placeholder_funcid]);
        if contains_placeholder || te.expr.collect_nodes::<pg_sys::Var>().is_empty() {
            continue;
        }
        grouping_slots.push((te.expr.cast(), (i + 1) as pg_sys::AttrNumber));
    }

    // Context for the placeholder mutator (defined inside function since only used here)
    struct PlaceholderContext<'a> {
        current_te_idx: usize,
        placeholder_funcid: pg_sys::Oid,
        columns: &'a mut PlaceholderColumns,
        placeholders: Vec<Option<(PlaceholderColumn, pg_sys::Oid)>>,
        grouping_slots: &'a [(*mut pg_sys::Node, pg_sys::AttrNumber)],
    }

    #[pg_guard]
    unsafe extern "C-unwind" fn placeholder_mutator(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> *mut pg_sys::Node {
        if node.is_null() {
            return std::ptr::null_mut();
        }

        let ctx = &mut *(context as *mut PlaceholderContext);

        // If this is our placeholder FuncExpr, replace it with a placeholder Var
        if (*node).type_ == pg_sys::NodeTag::T_FuncExpr {
            let funcexpr = node as *mut pg_sys::FuncExpr;
            if (*funcexpr).funcid == ctx.placeholder_funcid {
                let result_type = (*funcexpr).funcresulttype;
                let (column, var) = ctx.columns.add(result_type, (*funcexpr).funccollid);
                debug_assert!(
                    ctx.placeholders[ctx.current_te_idx].is_none(),
                    "AggregateScan supports only one aggregate per target entry"
                );
                ctx.placeholders[ctx.current_te_idx] = Some((column, result_type));
                return var as *mut pg_sys::Node;
            }
        }

        // Mixed agg+group exprs: read a whole grouping expression from the virtual slot.
        if let Some(&(expr, slot_attr)) = ctx
            .grouping_slots
            .iter()
            .find(|&&(expr, _)| pg_sys::equal(node.cast(), expr.cast()))
        {
            return pg_sys::makeVar(
                pg_sys::INDEX_VAR,
                slot_attr,
                pg_sys::exprType(expr),
                pg_sys::exprTypmod(expr),
                pg_sys::exprCollation(expr),
                0,
            ) as *mut pg_sys::Node;
        }

        // For all other nodes, use the standard mutator to walk children
        #[cfg(not(any(feature = "pg16", feature = "pg17", feature = "pg18")))]
        {
            let fnptr = placeholder_mutator as *const ();
            let mutator: unsafe extern "C-unwind" fn() -> *mut pg_sys::Node =
                std::mem::transmute(fnptr);
            pg_sys::expression_tree_mutator(node, Some(mutator), context)
        }

        #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
        {
            pg_sys::expression_tree_mutator_impl(node, Some(placeholder_mutator), context)
        }
    }

    let list_len = targetlist_pg.len();
    let mut ctx = PlaceholderContext {
        current_te_idx: 0,
        placeholder_funcid,
        columns,
        placeholders: vec![None; list_len],
        grouping_slots: &grouping_slots,
    };

    // Build a new target list with ALL FuncExpr placeholders replaced by placeholder Vars.
    // This is critical for mixed wrapped/unwrapped cases like:
    //   SELECT pdb.agg(...), (pdb.agg(...))->'avg' FROM ...
    // If we only replace wrapped ones, ExecProject will try to execute the unwrapped
    // pdb.agg_fn() FuncExpr and fail with "placeholder should not be executed".
    let mut new_targetlist: *mut pg_sys::List = std::ptr::null_mut();
    for (i, te) in targetlist_pg.iter_ptr().enumerate() {
        let new_te = unsafe { pg_sys::flatCopyTargetEntry(te) };
        let te = unsafe { &*te };

        ctx.current_te_idx = i;

        // AggregateScan assumes a contiguous, non-resjunk targetlist.
        // Wrapper support doesn't change this assumption.
        debug_assert!(
            !te.resjunk,
            "AggregateScan does not support resjunk target entries"
        );
        debug_assert!(
            te.resno as usize == i + 1,
            "AggregateScan expects contiguous resno values (1, 2, 3, ...)"
        );

        // Check if this expression contains any placeholder FuncExpr (wrapped or top-level)
        let is_top_level_placeholder = unsafe { (*te.expr).type_ } == pg_sys::NodeTag::T_FuncExpr
            && unsafe { (*(te.expr as *mut pg_sys::FuncExpr)).funcid } == placeholder_funcid;

        let contains_placeholder =
            is_top_level_placeholder || te.expr.contains_functions(&[placeholder_funcid]);

        if contains_placeholder {
            // Replace ALL placeholder FuncExprs with placeholder Vars (both wrapped and top-level)
            // For top-level: the mutator will replace the FuncExpr directly with a Var
            // For wrapped: the mutator will walk the tree and replace nested FuncExprs
            let ctx_ptr = &mut ctx as *mut PlaceholderContext as *mut core::ffi::c_void;
            let new_expr = unsafe { placeholder_mutator(te.expr as *mut pg_sys::Node, ctx_ptr) };
            unsafe { (*new_te).expr = new_expr as *mut pg_sys::Expr };
        } else {
            // It's a grouping column!
            // The value for this column will be placed directly in the scan_slot at attribute (i + 1).
            // We must replace the original expression with an INDEX_VAR that reads from the virtual scan_slot,
            // otherwise ExecProject will try to evaluate the original expression (which usually refers
            // to the base relation tuple) against our virtual scan_slot and fail with "attribute number exceeds number of columns".
            let var = unsafe {
                pg_sys::makeVar(
                    pg_sys::INDEX_VAR,
                    (i + 1) as pg_sys::AttrNumber,
                    pg_sys::exprType(te.expr as *mut pg_sys::Node),
                    pg_sys::exprTypmod(te.expr as *mut pg_sys::Node),
                    pg_sys::exprCollation(te.expr as *mut pg_sys::Node),
                    0,
                )
            };
            unsafe { (*new_te).expr = var as *mut pg_sys::Expr };
        }

        new_targetlist = unsafe { pg_sys::lappend(new_targetlist, new_te.cast()) };
    }

    (new_targetlist, ctx.placeholders, true)
}

#[pg_extern(immutable, parallel_safe)]
pub unsafe fn placeholder_support(arg: Internal) -> ReturnedNodePointer {
    // We "simplify" calls to `pdb.score(<anyelement>)` by wrapping (a copy of) its `FuncExpr`
    // node in a `PlaceHolderVar`. This ensures that Postgres won't lose the scores when they're
    // emitted by our custom scan from underneath:
    // - JOIN nodes (Hash Join, Merge Join, etc.)
    // - Gather nodes (parallel aggregation)
    //
    // Without PlaceHolderVar, PostgreSQL may decide to re-evaluate the score function at a higher
    // level (e.g., in the Aggregate node above a Gather), which fails because scores can only be
    // computed within our Custom Scan execution context.
    if let Some(srs) = nodecast!(
        SupportRequestSimplify,
        T_SupportRequestSimplify,
        arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>()
    ) {
        if (*srs).root.is_null() {
            return ReturnedNodePointer::unsupported();
        }

        let root = (*srs).root;
        let has_aggs = !(*root).parse.is_null() && (*(*root).parse).hasAggs;

        // We walk the jointree instead of checking hasJoinRTEs because
        // anti/semi-joins (from NOT EXISTS/EXISTS sublinks pulled up by
        // pull_up_sublinks) create JoinExpr nodes without setting hasJoinRTEs.
        let has_joins = !(*root).parse.is_null()
            && (*(*root).parse).jointree.any(|node| {
                (*node).type_ == pg_sys::NodeTag::T_JoinExpr
                    // Comma joins have multiple fromlist entries without a JoinExpr.
                    || nodecast!(FromExpr, T_FromExpr, node).is_some_and(|from_expr| {
                        PgList::<pg_sys::Node>::from_pg((*from_expr).fromlist).len() > 1
                    })
            });

        if !has_joins && !has_aggs {
            // No joins and no aggregates - PlaceHolderVar provides no benefit
            return ReturnedNodePointer::unsupported();
        }

        let mut vars = (*srs).fcall.collect_nodes::<pg_sys::Var>();
        assert!(vars.len() == 1, "function is improperly defined or called");
        let var = vars.pop().unwrap();

        let phrels = pg_sys::bms_make_singleton((*var).varno as _);
        let phv = pg_sys::submodules::ffi::pg_guard_ffi_boundary(|| {
            #[allow(improper_ctypes)]
            #[rustfmt::skip]
            unsafe extern "C-unwind" {
                fn make_placeholder_expr(root: *mut pg_sys::PlannerInfo, expr: *mut pg_sys::Expr, phrels: pg_sys::Relids) -> *mut pg_sys::PlaceHolderVar;
            }

            make_placeholder_expr(
                (*srs).root,
                pg_sys::copyObjectImpl((*srs).fcall.cast()).cast(),
                phrels,
            )
        });

        // copy these properties up from the Var to its placeholder
        (*phv).phlevelsup = (*var).varlevelsup;
        #[cfg(not(feature = "pg15"))]
        {
            (*phv).phnullingrels = (*var).varnullingrels;
        }

        return ReturnedNodePointer::from_node(phv.cast());
    }

    ReturnedNodePointer::unsupported()
}

/// find all [`pg_sys::FuncExpr`] nodes matching a set of known function Oids that also contain
/// a [`pg_sys::Var`] as an argument that the specified `rti` level.
///
/// Returns a [`Vec`] of the matching `FuncExpr`s and the argument `Var` that finally matched.  If
/// the function has multiple arguments that match, it's returned multiple times.
pub unsafe fn pullout_funcexprs(
    node: *mut pg_sys::Node,
    funcids: &[pg_sys::Oid],
    rti: i32,
    root: *mut pg_sys::PlannerInfo,
) -> Vec<(*mut pg_sys::FuncExpr, *mut pg_sys::Var, FieldName)> {
    let mut matches = Vec::new();
    node.walk(|node| {
        if let Some(funcexpr) = nodecast!(FuncExpr, T_FuncExpr, node)
            && funcids.contains(&(*funcexpr).funcid)
        {
            let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
            for arg in args.iter_ptr() {
                if let Some((var, fieldname)) =
                    find_one_var_and_fieldname(VarContext::Planner(root), arg)
                {
                    let same_layer = (*var).varno as i32 == rti
                        || (rte_is_partitioned(root, (*var).varno as pg_sys::Index)
                            && rte_is_parent(
                                root,
                                rti as pg_sys::Index,
                                (*var).varno as pg_sys::Index,
                            ));
                    if same_layer {
                        matches.push((funcexpr, var, fieldname));
                    }
                }
            }
            return WalkControl::SkipChildren;
        }
        WalkControl::Continue
    });
    matches
}

/// Index of a column in the slot of a [`PlaceholderProjection`].
pub type PlaceholderColumn = usize;

/// The types of the placeholder values (score, snippets, aggregates) that a scan gives to its
/// projection for each row.
///
/// The values live in a virtual slot, and the target list reads them through `OUTER_VAR`s.
/// A `Const` can't carry them: `ExecBuildProjectionInfo` copies its value, so the scan would
/// have to build the projection again for each row. Each build initializes every `SubPlan` of
/// the target list again, in per-tuple memory, and the plan state keeps a pointer to it.
#[derive(Default)]
pub struct PlaceholderColumns(Vec<pg_sys::Oid>);

impl PlaceholderColumns {
    /// Adds a column, and returns it together with a `Var` that reads it.
    pub unsafe fn add(
        &mut self,
        typoid: pg_sys::Oid,
        collation: pg_sys::Oid,
    ) -> (PlaceholderColumn, *mut pg_sys::Var) {
        self.0.push(typoid);
        let var = pg_sys::makeVar(
            pg_sys::OUTER_VAR,
            self.0.len() as pg_sys::AttrNumber,
            typoid,
            -1,
            collation,
            0,
        );
        (self.0.len() - 1, var)
    }

    /// Builds the projection of `targetlist`, which reads these columns through the `Var`s that
    /// [`Self::add`] returned.
    ///
    /// # Safety
    /// `targetlist` and the current memory context must live as long as `planstate`.
    pub unsafe fn build_projection(
        &self,
        targetlist: *mut pg_sys::List,
        planstate: *mut pg_sys::PlanState,
        scan_tupdesc: pg_sys::TupleDesc,
    ) -> PlaceholderProjection {
        let tupdesc = pg_sys::CreateTemplateTupleDesc(self.0.len() as _);
        for (i, typoid) in self.0.iter().enumerate() {
            pg_sys::TupleDescInitEntry(
                tupdesc,
                (i + 1) as pg_sys::AttrNumber,
                std::ptr::null(),
                *typoid,
                -1,
                0,
            );
        }
        let slot =
            pg_sys::ExecInitExtraTupleSlot((*planstate).state, tupdesc, &pg_sys::TTSOpsVirtual);
        // The slot always holds a valid virtual tuple, so `PlaceholderProjection::set` can write
        // into its arrays directly.
        pg_sys::ExecStoreAllNullTuple(slot);

        let proj_info = pg_sys::ExecBuildProjectionInfo(
            targetlist,
            (*planstate).ps_ExprContext,
            (*planstate).ps_ResultTupleSlot,
            planstate,
            scan_tupdesc,
        );
        PlaceholderProjection { proj_info, slot }
    }
}

/// A projection that reads its placeholder values from a virtual slot. It's built one time for
/// the scan, see [`PlaceholderColumns`].
#[derive(Clone, Copy)]
pub struct PlaceholderProjection {
    proj_info: *mut pg_sys::ProjectionInfo,
    slot: *mut pg_sys::TupleTableSlot,
}

impl PlaceholderProjection {
    /// Sets the value of `column` for the next [`Self::project`]. `None` is `NULL`.
    #[inline(always)]
    pub unsafe fn set(&self, column: PlaceholderColumn, datum: Option<pg_sys::Datum>) {
        *(*self.slot).tts_values.add(column) = datum.unwrap_or_else(pg_sys::Datum::null);
        *(*self.slot).tts_isnull.add(column) = datum.is_none();
    }

    #[inline(always)]
    pub unsafe fn project(
        &self,
        scan_slot: *mut pg_sys::TupleTableSlot,
    ) -> *mut pg_sys::TupleTableSlot {
        let econtext = (*self.proj_info).pi_exprContext;
        (*econtext).ecxt_scantuple = scan_slot;
        (*econtext).ecxt_outertuple = self.slot;
        pg_sys::ExecProject(self.proj_info)
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub unsafe fn inject_placeholders(
    targetlist: *mut pg_sys::List,
    rti: pg_sys::Index,
    score_funcoids: [pg_sys::Oid; 2],
    snippet_funcoids: [pg_sys::Oid; 2],
    snippets_funcoids: [pg_sys::Oid; 2],
    snippet_positions_funcoids: [pg_sys::Oid; 2],
    attname_lookup: &HashMap<(Varno, pg_sys::AttrNumber), FieldName>,
    snippet_generators: &HashMap<SnippetType, Option<SnippetGenerator>>,
    columns: &mut PlaceholderColumns,
) -> (
    *mut pg_sys::List,
    PlaceholderColumn,
    HashMap<SnippetType, Vec<PlaceholderColumn>>,
) {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        context: *mut std::ffi::c_void,
    ) -> *mut pg_sys::Node {
        if node.is_null() {
            return std::ptr::null_mut();
        }

        #[inline(always)]
        unsafe fn inner(node: *mut pg_sys::Node, data: &mut Data) -> Option<*mut pg_sys::Node> {
            let funcexpr = nodecast!(FuncExpr, T_FuncExpr, node)?;

            if data.score_funcoids.contains(&(*funcexpr).funcid) {
                return Some(data.score_var.cast());
            }

            let mut this_snippet_type = None;

            if let Some(snippet_type) = extract_snippet(
                funcexpr,
                data.rti,
                data.snippet_funcoids,
                data.attname_lookup,
            ) {
                this_snippet_type = Some(snippet_type);
            }

            if let Some(snippet_type) = extract_snippets(
                funcexpr,
                data.rti,
                data.snippets_funcoids,
                data.attname_lookup,
            ) {
                this_snippet_type = Some(snippet_type);
            }

            if let Some(snippet_type) = extract_snippet_positions(
                funcexpr,
                data.rti,
                data.snippet_positions_funcoids,
                data.attname_lookup,
            ) {
                this_snippet_type = Some(snippet_type);
            }

            if let Some(this_snippet_type) = this_snippet_type {
                for snippet_type in data.snippet_generators.keys() {
                    if this_snippet_type == *snippet_type {
                        let (column, var) = data
                            .columns
                            .add(snippet_type.nodeoid(), pg_sys::DEFAULT_COLLATION_OID);

                        data.snippet_placeholders
                            .entry(snippet_type.clone())
                            .or_default()
                            .push(column);

                        return Some(var.cast());
                    }
                }
            }

            None
        }

        let data = &mut *context.cast::<Data>();
        if let Some(replacement) = inner(node, data) {
            return replacement;
        }

        #[cfg(not(any(feature = "pg16", feature = "pg17", feature = "pg18")))]
        {
            let fnptr = walker as *const ();
            let walker: unsafe extern "C-unwind" fn() -> *mut pg_sys::Node =
                std::mem::transmute(fnptr);
            pg_sys::expression_tree_mutator(node, Some(walker), context)
        }

        #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
        {
            pg_sys::expression_tree_mutator_impl(node, Some(walker), context)
        }
    }

    struct Data<'a> {
        rti: pg_sys::Index,

        score_funcoids: [pg_sys::Oid; 2],
        score_var: *mut pg_sys::Var,

        snippet_funcoids: [pg_sys::Oid; 2],
        snippets_funcoids: [pg_sys::Oid; 2],
        snippet_positions_funcoids: [pg_sys::Oid; 2],
        attname_lookup: &'a HashMap<(Varno, pg_sys::AttrNumber), FieldName>,

        snippet_generators: &'a HashMap<SnippetType, Option<SnippetGenerator>>,
        columns: &'a mut PlaceholderColumns,
        snippet_placeholders: HashMap<SnippetType, Vec<PlaceholderColumn>>,
    }

    let (score_placeholder, score_var) = columns.add(pg_sys::FLOAT4OID, pg_sys::Oid::INVALID);
    let mut data = Data {
        rti,

        score_funcoids,
        score_var,

        snippet_funcoids,
        snippets_funcoids,
        snippet_positions_funcoids,
        attname_lookup,
        snippet_generators,
        columns,
        snippet_placeholders: Default::default(),
    };
    let targetlist = walker(targetlist.cast(), addr_of_mut!(data).cast());
    (
        targetlist.cast(),
        score_placeholder,
        data.snippet_placeholders,
    )
}
