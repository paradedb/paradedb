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

//! Planner half of [`super`] (see the module docs there): qual collection,
//! candidate index scoring, `BitmapHeapPath` construction, and the
//! covered-HeapFilter query rewrite.

use crate::gucs;
use crate::postgres::customscan::CustomScan;
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::qual_inspect::{PlannerContext, Qual};
use crate::postgres::deparse::deparse_expr;
use crate::postgres::planner_warnings::add_planner_warning;
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;
use pgrx::{PgList, pg_sys};

pub struct BitmapPlanner {
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    bm25_oid: pg_sys::Oid,
    heap_exprs: Vec<*mut pg_sys::Node>,
    bm25_row_estimate: Option<f64>,
}

struct HarvestedClause {
    index_name: String,
    clause: *mut pg_sys::Node,
    lossy: bool,
    matched_heap_expr: bool,
}

/// One index whose bitmap can feed the intersection, carrying what the combination
/// step needs to score it and to tell it apart from a redundant one.
struct Candidate {
    ipath: *mut pg_sys::IndexPath,
    index_name: String,
    /// Cost of building this bitmap and converting it into the probe-able set.
    build_cost: f64,
    /// Fraction of the heap this index's quals select.
    selectivity: f64,
    /// Positions in `heap_exprs` this index's quals cover.
    covers: Vec<usize>,
}

/// A successful harvest: the path to attach as a `custom_paths` child, plus the
/// HeapExpr clauses its bitmap covers (with their planner-level lossiness).
pub struct HarvestedBitmap {
    pub path: *mut pg_sys::Path,
    /// Estimated cost of building the bitmap and converting it into the
    /// probe-able set; the owning scan should surface it as startup cost.
    pub build_cost: f64,
    covered: Vec<(*mut pg_sys::Node, bool)>,
}

impl HarvestedBitmap {
    /// Flag every HeapFilter covered by the bitmap and move filters for exactly-matched
    /// (non-lossy) clauses into `recheck_filters`: their predicate is proven by exact
    /// bitmap membership and only needs re-evaluation on lossy/recheck pages. Lossy
    /// clauses stay in `always_filters`, which run on every bitmap survivor.
    pub unsafe fn rewrite_query(&self, query: &mut SearchQueryInput) {
        let mut next_consumer_id = 0u32;
        query.visit(&mut |sqi| {
            if let SearchQueryInput::HeapFilter {
                always_filters,
                recheck_filters,
                uses_tid_bitmap,
                bitmap_consumer_id,
                ..
            } = sqi
            {
                let mut i = 0;
                while i < always_filters.len() {
                    let node = unsafe { always_filters[i].get_expression_node() };
                    let covered = self.covered.iter().find(|(clause, _)| unsafe {
                        pg_sys::equal(node.cast(), (*clause).cast())
                    });
                    match covered {
                        Some((_, lossy)) => {
                            *uses_tid_bitmap = true;
                            if !*lossy {
                                recheck_filters.push(always_filters.remove(i));
                                continue;
                            }
                            i += 1;
                        }
                        None => i += 1,
                    }
                }
                // Each covered node is one claim-table consumer; its scorers claim
                // one cursor per segment.
                if *uses_tid_bitmap && bitmap_consumer_id.is_none() {
                    *bitmap_consumer_id = Some(next_consumer_id);
                    next_consumer_id += 1;
                }
            }
        });
    }
}

impl BitmapPlanner {
    /// Collect the expr node of every `Qual::HeapExpr` reachable through top-level AND
    /// structure. Returns `None` when there is nothing to cover, or when a HeapExpr
    /// appears under OR/NOT, where its bitmap could not be used to reject rows.
    /// (Covering OR clauses at whole-rinfo granularity via BitmapOr is future work.)
    pub fn from_query(
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        bm25_oid: pg_sys::Oid,
        quals: &Qual,
        bm25_row_estimate: Option<f64>,
    ) -> Option<Self> {
        fn collect(qual: &Qual, out: &mut Vec<*mut pg_sys::Node>) -> bool {
            match qual {
                Qual::HeapExpr { expr_node, .. } => {
                    out.push(*expr_node);
                    true
                }
                Qual::And(qs) => qs.iter().all(|q| collect(q, out)),
                Qual::Or(_) | Qual::Not(_) => !qual.contains_heap_expr(),
                _ => true,
            }
        }

        let mut heap_exprs = Vec::new();
        if !collect(quals, &mut heap_exprs) || heap_exprs.is_empty() {
            return None;
        }
        Some(Self {
            root,
            rel,
            bm25_oid,
            heap_exprs,
            bm25_row_estimate,
        })
    }

    /// Like `from_query`, but for callers that only have the built `SearchQueryInput`
    /// (the Aggregate Scan). HeapFilter expressions are collected through AND-position
    /// structure (`must` chains and score-neutral wrappers); a HeapFilter under
    /// `should`/`must_not` disqualifies for the same reason OR/NOT do in `from_query`.
    pub unsafe fn from_search_query(
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        bm25_oid: pg_sys::Oid,
        query: &SearchQueryInput,
        bm25_row_estimate: Option<f64>,
    ) -> Option<Self> {
        unsafe fn collect(sqi: &SearchQueryInput, out: &mut Vec<*mut pg_sys::Node>) -> bool {
            unsafe {
                match sqi {
                    SearchQueryInput::HeapFilter {
                        indexed_query,
                        always_filters,
                        ..
                    } => {
                        for filter in always_filters {
                            let node = filter.get_expression_node();
                            if !node.is_null() {
                                out.push(node);
                            }
                        }
                        collect(indexed_query, out)
                    }
                    SearchQueryInput::Boolean {
                        must,
                        should,
                        must_not,
                        ..
                    } => {
                        must.iter().all(|q| collect(q, out))
                            && !should
                                .iter()
                                .chain(must_not.iter())
                                .any(|q| q.has_heap_filters())
                    }
                    SearchQueryInput::Boost { query, .. }
                    | SearchQueryInput::ConstScore { query, .. }
                    | SearchQueryInput::WithIndex { query, .. } => collect(query, out),
                    SearchQueryInput::ScoreFilter { query, .. } => {
                        query.as_ref().is_none_or(|q| collect(q, out))
                    }
                    other => !other.has_heap_filters(),
                }
            }
        }

        let mut heap_exprs = Vec::new();
        if rel.is_null() || unsafe { !collect(query, &mut heap_exprs) } || heap_exprs.is_empty() {
            return None;
        }
        Some(Self {
            root,
            rel,
            bm25_oid,
            heap_exprs,
            bm25_row_estimate,
        })
    }

    /// Build a `BitmapHeapPath` usable as an intersection source. Always self-built:
    /// scavenging the rel's pathlists is unsound because `add_path` pfrees dominated
    /// non-IndexPath paths, so a harvested pointer could be freed before plan creation.
    pub unsafe fn harvest(&self) -> Option<HarvestedBitmap> {
        if !gucs::enable_bitmap_intersection() {
            return None;
        }
        unsafe {
            let harvested = self
                .build_bitmap_path()
                .and_then(|(bhp, build_cost)| self.accept(bhp, build_cost))?;
            let bm25 = PgSearchRelation::open(self.bm25_oid);
            if !bm25.is_ctid_sorted_asc() {
                let covered = harvested
                    .covered
                    .iter()
                    .map(|(clause, _)| {
                        deparse_expr(
                            Some(&PlannerContext::from_planner(self.root)),
                            &bm25,
                            *clause,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" AND ");
                // No table context: the scan itself plans successfully, and a
                // successful plan clears context-keyed warnings for its alias.
                add_planner_warning(
                    "Bitmap Intersection",
                    format!(
                        "a faster query path is likely available for {covered}, but index \
                         \"{}\" is not sorted by ctid so a slower path is being used; recreate \
                         the index with sort_by = 'ctid' to enable the faster path. To disable \
                         this warning: SET paradedb.planner_warnings = 'off'",
                        bm25.name()
                    ),
                    (),
                );
                return None;
            }
            Some(harvested)
        }
    }

    /// Accept or reject one candidate: walk its bitmapqual tree enforcing the
    /// module-level scope rules, and require that it covers at least one of our
    /// HeapExpr clauses.
    unsafe fn accept(
        &self,
        bhp: *mut pg_sys::BitmapHeapPath,
        build_cost: f64,
    ) -> Option<HarvestedBitmap> {
        unsafe {
            // A parameterized path's index quals reference outer rels or nestloop
            // params the custom scan's plan context never supplies.
            if !(*bhp).path.param_info.is_null() {
                return None;
            }
            let mut clauses = Vec::new();
            let mut pending = vec![(*bhp).bitmapqual];
            while let Some(path) = pending.pop() {
                match (*path).type_ {
                    pg_sys::NodeTag::T_BitmapAndPath => {
                        let and_path: *mut pg_sys::BitmapAndPath = path.cast();
                        pending.extend(
                            PgList::<pg_sys::Path>::from_pg((*and_path).bitmapquals).iter_ptr(),
                        );
                    }
                    pg_sys::NodeTag::T_IndexPath => {
                        let ip: *mut pg_sys::IndexPath = path.cast();
                        let indexoid = (*(*ip).indexinfo).indexoid;
                        if indexoid == self.bm25_oid {
                            pgrx::debug1!(
                                "[bitmap_intersection] skipping BitmapHeapPath: bitmapqual is over the ParadeDB index itself"
                            );
                            return None;
                        }
                        let index_name = PgSearchRelation::open(indexoid).name().to_string();
                        for iclause in
                            PgList::<pg_sys::IndexClause>::from_pg((*ip).indexclauses).iter_ptr()
                        {
                            let clause = (*(*iclause).rinfo).clause.cast::<pg_sys::Node>();
                            clauses.push(HarvestedClause {
                                index_name: index_name.clone(),
                                clause,
                                lossy: (*iclause).lossy,
                                matched_heap_expr: self.matches_heap_expr(clause),
                            });
                        }
                    }
                    unsupported => {
                        // TODO: `BitmapOr` is not supported yet
                        pgrx::debug1!(
                            "[bitmap_intersection] skipping BitmapHeapPath: bitmapqual contains {:?}",
                            unsupported
                        );
                        return None;
                    }
                }
            }

            if !clauses.iter().any(|c| c.matched_heap_expr) {
                pgrx::debug1!(
                    "[bitmap_intersection] skipping BitmapHeapPath: covers no heap_filter clause"
                );
                return None;
            }

            for clause in &clauses {
                pgrx::debug1!(
                    "[bitmap_intersection] index={} lossy={} matched_heap_expr={}",
                    clause.index_name,
                    clause.lossy,
                    clause.matched_heap_expr,
                );
            }
            pgrx::debug1!(
                "[bitmap_intersection] harvested BitmapHeapPath: {} index clause(s), {} matched heap_filter clause(s), est rows={:.0}",
                clauses.len(),
                clauses.iter().filter(|c| c.matched_heap_expr).count(),
                (*bhp).path.rows,
            );
            Some(HarvestedBitmap {
                path: bhp.cast(),
                build_cost,
                covered: clauses
                    .into_iter()
                    .filter(|c| c.matched_heap_expr)
                    .map(|c| (c.clause, c.lossy))
                    .collect(),
            })
        }
    }

    /// Build a `BitmapHeapPath` over the non-ParadeDB indexes whose bitmaps are worth
    /// intersecting: every index with a key column matching a HeapExpr clause is scored
    /// by [`Self::ledger`], and the positive-net ones are combined with a `BitmapAnd`
    /// in descending net order while each addition still pays for itself. Without a
    /// ParadeDB-side row estimate the first workable index is taken unscored.
    unsafe fn build_bitmap_path(&self) -> Option<(*mut pg_sys::BitmapHeapPath, f64)> {
        unsafe {
            let mut candidates = Vec::new();
            for ioi in PgList::<pg_sys::IndexOptInfo>::from_pg((*self.rel).indexlist).iter_ptr() {
                let Some(candidate) = self.candidate(ioi) else {
                    continue;
                };
                // With no ParadeDB-side estimate there is nothing to score against, so
                // the first workable index is taken unscored and uncombined.
                if self.bm25_row_estimate.is_none() {
                    return Some((self.heap_path(candidate.ipath.cast()), candidate.build_cost));
                }
                candidates.push(candidate);
            }
            let bm25_row_estimate = self.bm25_row_estimate?;
            if candidates.is_empty() {
                return None;
            }

            // Constant across candidates, so it is evaluated once for the whole pass.
            let per_row_saved = self.per_row_saved();
            let mut scored = Vec::with_capacity(candidates.len());
            for candidate in candidates {
                let net = self.ledger(&candidate, bm25_row_estimate, per_row_saved);
                pgrx::debug1!(
                    "[bitmap_intersection] index {}: net={net:.2} (bm25_row_estimate={bm25_row_estimate:.0} selectivity={:.6} indextotalcost={:.2})",
                    candidate.index_name,
                    candidate.selectivity,
                    (*candidate.ipath).indextotalcost,
                );
                scored.push((candidate, net));
            }

            // Best net first, then keep adding while the next bitmap still pays for
            // itself: it only sees the rows its predecessors kept, so its benefit
            // shrinks by their combined selectivity while its build cost stays full
            // price. Sorted descending, so the first non-paying one ends the run.
            scored.sort_by(|(_, a), (_, b)| b.total_cmp(a));
            let mut chosen: Vec<Candidate> = Vec::new();
            let mut covered: Vec<usize> = Vec::new();
            let mut reachable_rows = bm25_row_estimate;
            for (candidate, _) in scored {
                // Any shared clause disqualifies the candidate, as Postgres does
                // with `bms_overlap` in `choose_bitmap_and`. The ledger multiplies
                // selectivities as if the bitmaps were independent, so a clause both
                // indexes answer is counted twice and makes the addition look like it
                // rejects rows the first bitmap already rejected.
                if candidate.covers.iter().any(|c| covered.contains(c)) {
                    pgrx::debug1!(
                        "[bitmap_intersection] index {}: shares a clause with the accepted set, skipping",
                        candidate.index_name
                    );
                    continue;
                }
                let incremental = self.ledger(&candidate, reachable_rows, per_row_saved);
                pgrx::debug1!(
                    "[bitmap_intersection] index {}: incremental net={incremental:.2} against {} accepted bitmap(s) (reachable_rows={reachable_rows:.0})",
                    candidate.index_name,
                    chosen.len(),
                );
                if incremental <= 0.0 {
                    break;
                }
                reachable_rows *= candidate.selectivity;
                covered.extend(&candidate.covers);
                chosen.push(candidate);
            }

            let (first, rest) = chosen.split_first()?;
            let build_cost = chosen.iter().map(|c| c.build_cost).sum();
            let bitmapqual = if rest.is_empty() {
                first.ipath.cast::<pg_sys::Path>()
            } else {
                let mut bitmapquals = PgList::<pg_sys::Path>::new();
                for candidate in &chosen {
                    bitmapquals.push(candidate.ipath.cast());
                }
                pg_sys::create_bitmap_and_path(self.root, self.rel, bitmapquals.into_pg()).cast()
            };
            Some((self.heap_path(bitmapqual), build_cost))
        }
    }

    /// Score one index as an intersection source. `None` when it cannot produce a
    /// usable bitmap, when none of its key columns matches a HeapExpr clause, or when
    /// its bitmap would overflow `work_mem`.
    unsafe fn candidate(&self, ioi: *mut pg_sys::IndexOptInfo) -> Option<Candidate> {
        unsafe {
            if (*ioi).indexoid == self.bm25_oid || !(*ioi).amhasgetbitmap || (*ioi).hypothetical {
                return None;
            }
            // Partial indexes need predicate-implication checks.
            if !(*ioi).indpred.is_null() {
                return None;
            }

            let mut matched = Vec::new();
            let mut covers = Vec::new();
            for ri in PgList::<pg_sys::RestrictInfo>::from_pg((*ioi).indrestrictinfo).iter_ptr() {
                if let Some(heap_expr) = self.heap_expr_index((*ri).clause.cast())
                    && let Some(iclause) = IndexClause::from_clause(self.root, ri, ioi)
                {
                    matched.push(iclause);
                    covers.push(heap_expr);
                }
            }
            if matched.is_empty() {
                return None;
            }
            // `IndexPath.indexclauses` must be ordered by index column; the
            // clauses above accumulate in `indrestrictinfo` order.
            matched.sort_by_key(IndexClause::indexcol);
            let mut iclauses = PgList::<pg_sys::IndexClause>::new();
            for iclause in matched {
                iclauses.push(iclause.into_pg());
            }
            let ipath = pg_sys::create_index_path(
                self.root,
                ioi,
                iclauses.into_pg(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                pg_sys::ScanDirection::ForwardScanDirection,
                false,
                std::ptr::null_mut(),
                1.0,
                false,
            );
            let index_name = PgSearchRelation::open((*ioi).indexoid).name().to_string();
            if self.overflows_work_mem(ipath) {
                pgrx::debug1!(
                    "[bitmap_intersection] index {index_name}: bitmap would overflow work_mem, skipping"
                );
                return None;
            }
            Some(Candidate {
                ipath,
                index_name,
                build_cost: self.bitmap_build_cost(ipath),
                selectivity: selectivity(ipath),
                covers,
            })
        }
    }

    /// Estimated number of TIDs this index's bitmap holds.
    unsafe fn bitmap_rows(&self, ipath: *mut pg_sys::IndexPath) -> f64 {
        unsafe { selectivity(ipath) * (*self.rel).tuples.max(1.0) }
    }

    /// Cost of producing the probe-able set: the index scan that builds the
    /// TIDBitmap plus converting its entries.
    unsafe fn bitmap_build_cost(&self, ipath: *mut pg_sys::IndexPath) -> f64 {
        unsafe {
            let cpu_operator_cost = *std::ptr::addr_of!(pg_sys::cpu_operator_cost);
            (*ipath).indextotalcost + self.bitmap_rows(ipath) * 0.1 * cpu_operator_cost
        }
    }

    /// Cost avoided for every row a bitmap rejects: the heap fetch it skips plus each
    /// heap filter it no longer evaluates. Independent of which index produced the
    /// bitmap, so callers scoring several indexes hoist it out of their loop.
    unsafe fn per_row_saved(&self) -> f64 {
        unsafe {
            let tuples = (*self.rel).tuples.max(1.0);
            let pages = (*self.rel).pages as f64;

            let cpu_tuple_cost = *std::ptr::addr_of!(pg_sys::cpu_tuple_cost);
            let random_page_cost = *std::ptr::addr_of!(pg_sys::random_page_cost);

            let mut qual_cost = pg_sys::QualCost::default();
            let mut exprs = PgList::<pg_sys::Node>::new();
            for expr in &self.heap_exprs {
                exprs.push(*expr);
            }
            pg_sys::cost_qual_eval(&mut qual_cost, exprs.into_pg(), self.root);

            cpu_tuple_cost + qual_cost.per_tuple + (pages / tuples) * random_page_cost
        }
    }

    /// Net benefit of probing this bitmap ahead of the heap filters.
    ///
    /// Benefit: each row the bitmap rejects (the non-selected fraction of the
    /// `reachable_rows` that still arrive at it) skips its heap fetch and all heap
    /// filter evaluation. `reachable_rows` is the full ParadeDB-side estimate for the
    /// first bitmap in an intersection, and the fraction its predecessors kept for
    /// every bitmap after it.
    ///
    /// Cost: building the bitmap (`indextotalcost`) plus converting its entries into
    /// the probe-able set, which does not shrink as the intersection grows.
    fn ledger(&self, candidate: &Candidate, reachable_rows: f64, per_row_saved: f64) -> f64 {
        reachable_rows * (1.0 - candidate.selectivity) * per_row_saved - candidate.build_cost
    }

    /// A bitmap whose estimated TID count can't fit in `work_mem` degrades to lossy
    /// pages, which cannot reject anything.
    unsafe fn overflows_work_mem(&self, ipath: *mut pg_sys::IndexPath) -> bool {
        unsafe {
            let work_mem_kb = *std::ptr::addr_of!(pg_sys::work_mem) as f64;
            self.bitmap_rows(ipath) * 8.0 > work_mem_kb * 1024.0
        }
    }

    unsafe fn heap_path(&self, bitmapqual: *mut pg_sys::Path) -> *mut pg_sys::BitmapHeapPath {
        unsafe {
            pg_sys::create_bitmap_heap_path(
                self.root,
                self.rel,
                bitmapqual,
                (*self.rel).lateral_relids,
                1.0,
                0,
            )
        }
    }

    /// Position in [`Self::heap_exprs`] of the HeapExpr this clause is, if any.
    unsafe fn heap_expr_index(&self, clause: *mut pg_sys::Node) -> Option<usize> {
        unsafe {
            self.heap_exprs
                .iter()
                .position(|expr| pg_sys::equal(clause.cast(), (*expr).cast()))
        }
    }

    unsafe fn matches_heap_expr(&self, clause: *mut pg_sys::Node) -> bool {
        unsafe { self.heap_expr_index(clause).is_some() }
    }
}

/// `create_plan_recurse` can't emit a bare `BitmapIndexScan`, so a harvested bitmap
/// path arrives planned as a full BitmapHeapScan; keep only its bitmap-producing
/// subtree (`BitmapIndexScan` / `BitmapAnd`). Shared by every scan type that carries a
/// harvested child.
pub unsafe fn keep_bitmap_child_plan<CS: CustomScan>(builder: &mut CustomScanBuilder<CS>) {
    unsafe {
        let custom_plans = PgList::<pg_sys::Plan>::from_pg(builder.custom_plans());
        if let Some(child) = custom_plans.get_ptr(0)
            && (*child).type_ == pg_sys::NodeTag::T_BitmapHeapScan
        {
            let mut replacement = PgList::<pg_sys::Plan>::new();
            replacement.push((*child).lefttree);
            builder.set_custom_plans(replacement.into_pg());
        }
    }
}

struct IndexClause(*mut pg_sys::IndexClause);

impl IndexClause {
    /// Match one restriction clause against every key column of `ioi`.
    unsafe fn from_clause(
        root: *mut pg_sys::PlannerInfo,
        ri: *mut pg_sys::RestrictInfo,
        ioi: *mut pg_sys::IndexOptInfo,
    ) -> Option<Self> {
        unsafe {
            // Same gates as core's `match_clause_to_index`: pseudoconstant clauses
            // never become index quals, and a clause above the rel's minimum
            // security level (RLS / security-barrier quals) must not be evaluated
            // early by the index AM unless it is leakproof.
            if (*ri).pseudoconstant || !pg_sys::restriction_is_securely_promotable(ri, (*ioi).rel) {
                return None;
            }
            match (*(*ri).clause.cast::<pg_sys::Node>()).type_ {
                pg_sys::NodeTag::T_OpExpr => Self::from_opexpr(root, ri, ioi),
                pg_sys::NodeTag::T_FuncExpr => Self::from_funcexpr(root, ri, ioi),
                pg_sys::NodeTag::T_ScalarArrayOpExpr => Self::from_saop(ri, ioi),
                _ => None,
            }
        }
    }

    unsafe fn from_opexpr(
        root: *mut pg_sys::PlannerInfo,
        ri: *mut pg_sys::RestrictInfo,
        ioi: *mut pg_sys::IndexOptInfo,
    ) -> Option<Self> {
        unsafe {
            let opexpr = (*ri).clause.cast::<pg_sys::OpExpr>();
            let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
            if args.len() != 2 {
                return None;
            }
            let (leftop, rightop) = (
                strip_relabel(args.get_ptr(0)?),
                strip_relabel(args.get_ptr(1)?),
            );

            for indexcol in 0..(*ioi).nkeycolumns {
                if pg_sys::match_index_to_operand(leftop, indexcol, ioi)
                    && is_pseudoconstant(rightop)
                {
                    if Self::direct_match_ok((*opexpr).opno, (*opexpr).inputcollid, indexcol, ioi) {
                        return Some(Self::direct(ri, ri, indexcol));
                    }
                    if let Some(iclause) =
                        Self::from_support_function(root, ri, (*opexpr).opfuncid, 0, indexcol, ioi)
                    {
                        return Some(iclause);
                    }
                }

                if pg_sys::match_index_to_operand(rightop, indexcol, ioi)
                    && is_pseudoconstant(leftop)
                {
                    let comm_op = pg_sys::get_commutator((*opexpr).opno);
                    if comm_op != pg_sys::InvalidOid
                        && Self::direct_match_ok(comm_op, (*opexpr).inputcollid, indexcol, ioi)
                    {
                        let commuted = pg_sys::commute_restrictinfo(ri, comm_op);
                        return Some(Self::direct(ri, commuted, indexcol));
                    }
                    if let Some(iclause) =
                        Self::from_support_function(root, ri, (*opexpr).opfuncid, 1, indexcol, ioi)
                    {
                        return Some(iclause);
                    }
                }
            }
            None
        }
    }

    /// Mirrors core's `match_saopclause_to_indexcol`: `key = ANY(array)` answers from the
    /// index when the array is a pseudoconstant. `ALL` is rejected because the index
    /// cannot answer a conjunction over the array in one scan, and there is no commuted
    /// form to try because the array can only be the right operand.
    unsafe fn from_saop(
        ri: *mut pg_sys::RestrictInfo,
        ioi: *mut pg_sys::IndexOptInfo,
    ) -> Option<Self> {
        unsafe {
            let saop = (*ri).clause.cast::<pg_sys::ScalarArrayOpExpr>();
            if !(*saop).useOr {
                return None;
            }
            let args = PgList::<pg_sys::Node>::from_pg((*saop).args);
            if args.len() != 2 {
                return None;
            }
            let (leftop, rightop) = (
                strip_relabel(args.get_ptr(0)?),
                strip_relabel(args.get_ptr(1)?),
            );
            if !is_pseudoconstant(rightop) {
                return None;
            }

            for indexcol in 0..(*ioi).nkeycolumns {
                if pg_sys::match_index_to_operand(leftop, indexcol, ioi)
                    && Self::direct_match_ok((*saop).opno, (*saop).inputcollid, indexcol, ioi)
                {
                    return Some(Self::direct(ri, ri, indexcol));
                }
            }
            None
        }
    }

    /// Gates every direct (non support function) match must clear: the operator belongs
    /// to the index column's opfamily, and the index collation matches the operator's
    /// input collation. Without the collation check the index answers under different
    /// comparison semantics and its bitmap can miss valid rows.
    unsafe fn direct_match_ok(
        opno: pg_sys::Oid,
        inputcollid: pg_sys::Oid,
        indexcol: i32,
        ioi: *mut pg_sys::IndexOptInfo,
    ) -> bool {
        unsafe {
            let idxcoll = *(*ioi).indexcollations.add(indexcol as usize);
            let collation_ok = idxcoll == pg_sys::InvalidOid || idxcoll == inputcollid;
            collation_ok && pg_sys::op_in_opfamily(opno, *(*ioi).opfamily.add(indexcol as usize))
        }
    }

    /// The `IndexClause` for a cleared direct match. `indexqual` is what the index AM
    /// evaluates: the clause itself, or its commuted form when the key is the right
    /// operand.
    unsafe fn direct(
        ri: *mut pg_sys::RestrictInfo,
        indexqual: *mut pg_sys::RestrictInfo,
        indexcol: i32,
    ) -> Self {
        unsafe {
            let mut indexquals = PgList::<pg_sys::RestrictInfo>::new();
            indexquals.push(indexqual);
            Self::new(ri, indexquals, false, indexcol)
        }
    }

    unsafe fn from_funcexpr(
        root: *mut pg_sys::PlannerInfo,
        ri: *mut pg_sys::RestrictInfo,
        ioi: *mut pg_sys::IndexOptInfo,
    ) -> Option<Self> {
        unsafe {
            let funcexpr = (*ri).clause.cast::<pg_sys::FuncExpr>();
            let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
            for indexcol in 0..(*ioi).nkeycolumns {
                for (indexarg, arg) in args.iter_ptr().enumerate() {
                    if pg_sys::match_index_to_operand(strip_relabel(arg), indexcol, ioi)
                        && let Some(iclause) = Self::from_support_function(
                            root,
                            ri,
                            (*funcexpr).funcid,
                            indexarg as i32,
                            indexcol,
                            ioi,
                        )
                    {
                        return Some(iclause);
                    }
                }
            }
            None
        }
    }

    /// Support functions allow quals to be derived from an index, potentially lossily
    unsafe fn from_support_function(
        root: *mut pg_sys::PlannerInfo,
        ri: *mut pg_sys::RestrictInfo,
        funcid: pg_sys::Oid,
        indexarg: i32,
        indexcol: i32,
        ioi: *mut pg_sys::IndexOptInfo,
    ) -> Option<Self> {
        unsafe {
            let prosupport = pg_sys::get_func_support(funcid);
            if prosupport == pg_sys::InvalidOid {
                return None;
            }

            let mut req = pg_sys::SupportRequestIndexCondition {
                type_: pg_sys::NodeTag::T_SupportRequestIndexCondition,
                root,
                funcid,
                node: (*ri).clause.cast(),
                indexarg,
                index: ioi,
                indexcol,
                opfamily: *(*ioi).opfamily.add(indexcol as usize),
                indexcollation: *(*ioi).indexcollations.add(indexcol as usize),
                lossy: true,
            };

            let result = pg_sys::OidFunctionCall1Coll(
                prosupport,
                pg_sys::InvalidOid,
                pg_sys::Datum::from(std::ptr::addr_of_mut!(req) as usize),
            );
            let derived: *mut pg_sys::List = result.cast_mut_ptr();
            if derived.is_null() {
                return None;
            }

            let mut indexquals = PgList::<pg_sys::RestrictInfo>::new();
            for expr in PgList::<pg_sys::Node>::from_pg(derived).iter_ptr() {
                indexquals.push(make_simple_restrictinfo(root, expr.cast()));
            }
            if indexquals.is_empty() {
                return None;
            }
            Some(Self::new(ri, indexquals, req.lossy, indexcol))
        }
    }

    unsafe fn new(
        ri: *mut pg_sys::RestrictInfo,
        indexquals: PgList<pg_sys::RestrictInfo>,
        lossy: bool,
        indexcol: i32,
    ) -> Self {
        unsafe {
            let iclause: *mut pg_sys::IndexClause =
                pg_sys::palloc0(std::mem::size_of::<pg_sys::IndexClause>()).cast();
            (*iclause).type_ = pg_sys::NodeTag::T_IndexClause;
            (*iclause).rinfo = ri;
            (*iclause).indexquals = indexquals.into_pg();
            (*iclause).lossy = lossy;
            (*iclause).indexcol = indexcol as _;
            (*iclause).indexcols = std::ptr::null_mut();
            Self(iclause)
        }
    }

    fn indexcol(&self) -> pg_sys::AttrNumber {
        unsafe { (*self.0).indexcol }
    }

    fn into_pg(self) -> *mut pg_sys::IndexClause {
        self.0
    }
}

/// An index path's estimated selectivity, clamped to a usable fraction.
unsafe fn selectivity(ipath: *mut pg_sys::IndexPath) -> f64 {
    unsafe { (*ipath).indexselectivity.clamp(0.0, 1.0) }
}

unsafe fn strip_relabel(mut node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    unsafe {
        while (*node).type_ == pg_sys::NodeTag::T_RelabelType {
            node = (*node.cast::<pg_sys::RelabelType>()).arg.cast();
        }
        node
    }
}

unsafe fn is_pseudoconstant(node: *mut pg_sys::Node) -> bool {
    unsafe { !pg_sys::contain_var_clause(node) && !pg_sys::contain_volatile_functions(node) }
}

#[cfg(feature = "pg15")]
unsafe fn make_simple_restrictinfo(
    root: *mut pg_sys::PlannerInfo,
    clause: *mut pg_sys::Expr,
) -> *mut pg_sys::RestrictInfo {
    unsafe {
        pg_sys::make_restrictinfo(
            root,
            clause,
            true,
            false,
            false,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    }
}

#[cfg(not(feature = "pg15"))]
unsafe fn make_simple_restrictinfo(
    root: *mut pg_sys::PlannerInfo,
    clause: *mut pg_sys::Expr,
) -> *mut pg_sys::RestrictInfo {
    unsafe {
        pg_sys::make_restrictinfo(
            root,
            clause,
            true,
            false,
            false,
            false,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    }
}
